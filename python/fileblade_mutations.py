from __future__ import annotations

import base64
from dataclasses import dataclass
import json
import os
from pathlib import Path
import stat

from fileblade_paths import path_text

MAX_CONTENT = 8 * 1024 * 1024
MAX_INPUT = 24 * 1024 * 1024


def mutate(document: dict) -> None:
    from fileblade_process import run
    data = json.dumps(document, ensure_ascii=True, separators=(",", ":")).encode()
    if len(data) > MAX_INPUT:
        raise OSError("change exceeds the mutation input limit")
    core = Path(__file__).resolve().parents[1]
    binary = os.environ.get("FILEBLADE_BINARY") or str(core / "fileblade")
    result = run([binary, "_companion-mutate", "--output=json"], input=data,
                 input_limit=MAX_INPUT, timeout=6, stdout_limit=4096, stderr_limit=4096)
    try:
        response = json.loads(result.stdout)
    except (ValueError, UnicodeError):
        response = {}
    if result.returncode or result.stdout_truncated or response.get("ok") is not True:
        detail = response.get("error") or response.get("message")
        if not detail:
            try:
                failure = json.loads(result.stderr)
                detail = failure.get("error") or failure.get("message")
            except (ValueError, UnicodeError):
                pass
        raise OSError(str(detail or "change was not confirmed; refresh before retrying")[:400])


def directory(path: Path) -> int:
    descriptor = os.open("/", os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC)
    try:
        for part in path.parts[1:]:
            child = os.open(part, os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC | os.O_NOFOLLOW, dir_fd=descriptor)
            os.close(descriptor)
            descriptor = child
        return descriptor
    except BaseException:
        os.close(descriptor)
        raise


@dataclass
class Snapshot:
    logical: Path
    resolved: Path
    data: bytes | None
    version: dict | None

    @classmethod
    def read(cls, path: Path, limit: int) -> Snapshot:
        if not 0 <= limit <= MAX_CONTENT:
            raise OSError("configuration read limit is invalid")
        logical = Path(os.path.abspath(path))
        resolved = logical.resolve(strict=False)
        parent = -1
        descriptor = -1
        try:
            parent = directory(resolved.parent)
            descriptor = os.open(resolved.name, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW | os.O_NONBLOCK, dir_fd=parent)
            before = os.fstat(descriptor)
            if not stat.S_ISREG(before.st_mode) or before.st_size > limit:
                raise OSError("configuration is not a bounded regular file")
            chunks = []
            remaining = limit + 1
            while remaining:
                data = os.read(descriptor, min(65536, remaining))
                if not data:
                    break
                chunks.append(data)
                remaining -= len(data)
            data = b"".join(chunks)
            after = os.fstat(descriptor)
            if (len(data) > limit or (before.st_size, before.st_mtime_ns, before.st_ctime_ns)
                    != (after.st_size, after.st_mtime_ns, after.st_ctime_ns)
                    or logical.resolve(strict=True) != resolved):
                raise OSError("configuration changed while it was read")
            return cls(logical, resolved, data, {"dev":before.st_dev,"ino":before.st_ino,"data":base64.b64encode(data).decode("ascii")})
        except FileNotFoundError:
            if descriptor >= 0:
                raise OSError("configuration disappeared while it was read") from None
            return cls(logical, resolved, None, None)
        finally:
            if descriptor >= 0:
                os.close(descriptor)
            if parent >= 0:
                os.close(parent)

    def write(self, data: bytes) -> None:
        if len(data) > MAX_CONTENT:
            raise OSError("updated configuration exceeds its byte limit")
        mutate({"kind":"write", "path":path_text(str(self.logical)), "resolved":path_text(str(self.resolved)),
                "expected":self.version, "data":base64.b64encode(data).decode("ascii")})


class Document(dict):
    def __init__(self, data: dict, snapshot: Snapshot):
        super().__init__(data)
        self.snapshot = snapshot


def link(path: Path | str, target: Path | str) -> None:
    path = Path(os.path.abspath(path))
    parent = path.parent.resolve(strict=False)
    mutate({"kind":"link", "path":path_text(str(path)), "parent":path_text(str(parent)),
            "target":path_text(str(Path(target).resolve(strict=True)))})


def unlink(path: Path | str, target: Path | str, expected: os.stat_result | None = None) -> None:
    path = Path(os.path.abspath(path))
    parent = path.parent.resolve(strict=True)
    descriptor = directory(parent)
    try:
        current = os.stat(path.name, dir_fd=descriptor, follow_symlinks=False)
        if (not stat.S_ISLNK(current.st_mode) or (expected is not None and
                (current.st_dev, current.st_ino) != (expected.st_dev, expected.st_ino))):
            raise OSError("selected link changed; nothing was removed")
        destination = os.readlink(path.name, dir_fd=descriptor)
        if (parent / destination).resolve(strict=False) != Path(target).resolve(strict=False):
            raise OSError("selected link points elsewhere; nothing was removed")
        mutate({"kind":"unlink", "path":path_text(str(path)), "parent":path_text(str(parent)),
                "dev":current.st_dev, "ino":current.st_ino})
    finally:
        os.close(descriptor)

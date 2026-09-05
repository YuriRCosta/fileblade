"""Bounded filesystem watch plans collected alongside provider discovery."""

from contextvars import ContextVar
import json
from pathlib import Path

from fileblade_paths import NativePath, wire

_current = ContextVar("fileblade_inventory_watch", default=None)
MAX_WATCH_PATHS = 512
MAX_WATCH_CANDIDATES = 2048
MAX_WATCH_BYTES = 64 * 1024
SCOPES = ("all", "user", "project")


def lane_rows(rows, scope, project_scopes, key="scope"):
    if scope == "project":
        return [row for row in rows if row.get(key) in project_scopes]
    if scope == "user":
        return [row for row in rows if row.get(key) not in project_scopes]
    return rows


class WatchPlan:
    def __init__(self):
        self.paths = set()
        self.candidates = set()
        self.truncated = False
        self._token = None

    def __enter__(self):
        self._token = _current.set(self)
        return self

    def __exit__(self, *_):
        _current.reset(self._token)

    def directory(self, path):
        if not path.is_absolute() or path in self.candidates:
            return
        if len(self.paths) >= MAX_WATCH_PATHS or len(self.candidates) >= MAX_WATCH_CANDIDATES:
            self.truncated = True
            return
        self.candidates.add(path)
        for _ in range(64):
            try:
                if path.is_dir():
                    self.paths.add(path.resolve())
                    return
            except (OSError, RuntimeError):
                pass
            if path == path.parent:
                return
            path = path.parent

    def finish(self, document):
        paths, size = [], 2
        for path in sorted(self.paths):
            native = NativePath(str(path))
            encoded_size = len(json.dumps(wire(native), ensure_ascii=True).encode("utf-8")) + 1
            if size + encoded_size > MAX_WATCH_BYTES:
                self.truncated = True
                break
            size += encoded_size
            paths.append(native)
        document["watchPaths"] = paths
        document["watchTruncated"] = self.truncated
        return document


def watch_path(path, *, directory=False):
    plan = _current.get()
    if plan is None:
        return
    path = Path(path)
    plan.directory(path.parent)
    if directory:
        plan.directory(path)
    else:
        try:
            if path.is_symlink():
                plan.directory(path.resolve().parent)
        except (OSError, RuntimeError):
            pass

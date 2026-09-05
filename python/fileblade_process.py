from __future__ import annotations

import os
import select
import selectors
import signal
import struct
import subprocess
import threading
import time
from dataclasses import dataclass
from typing import Callable

_RESULT = struct.Struct("!ii")
_CHUNK = 32 * 1024
_GRACE = 0.15
_MAX_INPUT = 1024 * 1024


@dataclass
class Result:
    returncode: int
    stdout: bytes
    stderr: bytes
    stdout_truncated: bool
    stderr_truncated: bool
    timed_out: bool = False


class Capture:
    def __init__(self, limit: int, tail: bool) -> None:
        self.limit = limit
        self.tail = tail
        self.data = bytearray()
        self.truncated = False

    def add(self, chunk: bytes) -> None:
        self.truncated |= len(self.data) + len(chunk) > self.limit
        if self.tail:
            self.data.extend(chunk)
            del self.data[:max(0, len(self.data) - self.limit)]
        else:
            self.data.extend(chunk[:max(0, self.limit - len(self.data))])


def read_input(stream, *, limit: int, timeout: float) -> bytes:
    if limit < 0 or timeout <= 0:
        raise ValueError("nonnegative input limit and positive timeout are required")
    data = bytearray()
    deadline = time.monotonic() + timeout
    while True:
        remaining = deadline - time.monotonic()
        if remaining <= 0 or not select.select([stream], [], [], remaining)[0]:
            raise ValueError("command input timed out")
        chunk = os.read(stream.fileno(), min(_CHUNK, limit + 1 - len(data)))
        if not chunk:
            return bytes(data)
        data.extend(chunk)
        if len(data) > limit:
            raise ValueError("command input exceeds its byte limit")


def _close(fd: int) -> None:
    try:
        os.close(fd)
    except OSError:
        pass


def _release(fd: int, owned: set[int]) -> None:
    if fd in owned:
        owned.remove(fd)
        _close(fd)


def _close_others(keep: set[int]) -> None:
    for name in os.listdir("/proc/self/fd"):
        fd = int(name)
        if fd not in keep:
            _close(fd)


def _signal_group(pid: int, sig: int) -> None:
    try:
        os.killpg(pid, sig)
    except ProcessLookupError:
        pass


def _children() -> list[int]:
    with open(f"/proc/self/task/{os.getpid()}/children", "rb") as source:
        data = source.read(256 * 1024)
    if data and not data[-1:].isspace():
        raise RuntimeError("descendant list exceeds cleanup bound")
    return [int(value) for value in data.split()]


def _exit_info(pid: int):
    return os.waitid(os.P_PID, pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)


def _owned_children(leader: int) -> list[int]:
    return [pid for pid in _children() if _exit_info(pid) is not None
            or os.getpgid(pid) == leader]


def _reap_descendants(leader: int) -> None:
    _signal_group(leader, signal.SIGTERM)
    deadline = time.monotonic() + _GRACE
    while time.monotonic() < deadline:
        remaining = _owned_children(leader)
        if all(_exit_info(pid) is not None for pid in remaining):
            break
        for pid in remaining:
            try:
                os.kill(pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
        time.sleep(0.01)
    _signal_group(leader, signal.SIGKILL)
    deadline = time.monotonic() + 1.0
    while True:
        children = _owned_children(leader)
        if not children:
            return
        for pid in children:
            try:
                os.kill(pid, signal.SIGKILL)
                os.waitpid(pid, os.WNOHANG)
            except (ChildProcessError, ProcessLookupError):
                pass
        if time.monotonic() >= deadline:
            raise RuntimeError("descendants did not terminate")
        if _owned_children(leader):
            time.sleep(0.005)


def _wait_child(pid: int, owner: int, deadline: float) -> tuple[int, int]:
    child_fd = os.pidfd_open(pid)
    try:
        with selectors.DefaultSelector() as poller:
            poller.register(owner, selectors.EVENT_READ)
            poller.register(child_fd, selectors.EVENT_READ)
            while True:
                info = _exit_info(pid)
                if info is not None:
                    code = info.si_status if info.si_code == os.CLD_EXITED else -info.si_status
                    return code, 0
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    return 124, 1
                if any(key.fd == owner for key, _ in poller.select(remaining)):
                    return 125, 2
    finally:
        _close(child_fd)


def _become_subreaper() -> None:
    import ctypes

    libc = ctypes.CDLL(None, use_errno=True)
    if libc.prctl(36, 1, 0, 0, 0) != 0:
        raise OSError(ctypes.get_errno(), "could not enable child reaping")


def _guard(command, env, owner, result_fd, input_fd, out_fd, err_fd, deadline) -> None:
    process = None
    code, reason = 127, 3
    try:
        signal.signal(signal.SIGTERM, signal.SIG_DFL)
        os.setsid()
        _close_others({owner, result_fd, input_fd, out_fd, err_fd})
        _become_subreaper()
        process = subprocess.Popen(command, env=env, stdin=input_fd, stdout=out_fd,
                                   stderr=err_fd, start_new_session=True, close_fds=True)
        for fd in {input_fd, out_fd, err_fd}:
            _close(fd)
        code, reason = _wait_child(process.pid, owner, deadline)
    except BaseException:
        code, reason = 127, 3
    finally:
        if process is not None:
            try:
                _reap_descendants(process.pid)
                process.returncode = code
            except BaseException:
                code, reason = 125, 4
        try:
            os.write(result_fd, _RESULT.pack(code, reason))
        except OSError:
            pass
        os._exit(0)


def _pump(output_fd: int, error_fd: int, result_fd: int, input_fd: int,
          data: bytes, stdout: Capture, stderr: Capture,
          on_stdout: Callable[[bytes], None] | None, owned: set[int]) -> tuple[int, int]:
    result = bytearray()
    offset = 0
    completed_at = None
    with selectors.DefaultSelector() as poller:
        for fd, kind in ((output_fd, "out"), (error_fd, "err"), (result_fd, "result")):
            os.set_blocking(fd, False)
            poller.register(fd, selectors.EVENT_READ, kind)
        if data:
            os.set_blocking(input_fd, False)
            poller.register(input_fd, selectors.EVENT_WRITE, "input")
        else:
            _release(input_fd, owned)
        while poller.get_map():
            remaining = None if completed_at is None else _GRACE - (time.monotonic() - completed_at)
            if remaining is not None and remaining <= 0:
                stdout.truncated |= any(key.data == "out" for key in poller.get_map().values())
                stderr.truncated |= any(key.data == "err" for key in poller.get_map().values())
                return _RESULT.unpack(result)
            for key, _ in poller.select(remaining):
                if key.data == "input":
                    try:
                        offset += os.write(key.fd, data[offset:offset + _CHUNK])
                    except BrokenPipeError:
                        offset = len(data)
                    if offset == len(data):
                        poller.unregister(key.fd)
                        _release(key.fd, owned)
                    continue
                chunk = os.read(key.fd, _CHUNK)
                if not chunk:
                    poller.unregister(key.fd)
                elif key.data == "out":
                    stdout.add(chunk)
                    if on_stdout:
                        on_stdout(chunk)
                elif key.data == "err":
                    stderr.add(chunk)
                else:
                    result.extend(chunk)
                    if len(result) > _RESULT.size:
                        raise RuntimeError("invalid command supervisor result")
                    if len(result) == _RESULT.size:
                        completed_at = time.monotonic()
                        if _RESULT.unpack(result)[1] == 4:
                            return _RESULT.unpack(result)
    if len(result) != _RESULT.size:
        raise RuntimeError("command supervisor exited without a result")
    return _RESULT.unpack(result)


def run(command: list[str], *, timeout: float, stdout_limit: int,
        stderr_limit: int = 4096, tail: bool = False, merge_stderr: bool = False,
        input: bytes = b"", env: dict[str, str] | None = None, input_limit: int = _MAX_INPUT,
        on_stdout: Callable[[bytes], None] | None = None) -> Result:
    if not command or timeout <= 0 or min(stdout_limit, stderr_limit) < 0:
        raise ValueError("command, positive timeout and nonnegative output limits are required")
    if input_limit < 0 or input_limit > 24 * 1024 * 1024 or len(input) > input_limit:
        raise ValueError("command input exceeds its byte limit")
    if threading.active_count() != 1:
        raise RuntimeError("command helpers must run on a single-threaded process")
    pipes: list[tuple[int, int]] = []
    owned: set[int] = set()
    guard = None
    previous_term = None
    terminating = False
    try:
        for _ in range(5):
            pair = os.pipe()
            pipes.append(pair)
            owned.update(pair)
        owner, result, stdin, stdout_pipe, stderr_pipe = pipes

        def terminate(signum, frame):
            nonlocal terminating
            terminating = True
            _release(owner[1], owned)

        previous_term = signal.signal(signal.SIGTERM, terminate)
        deadline = time.monotonic() + timeout
        guard = os.fork()
        if guard == 0:
            _guard(command, env, owner[0], result[1], stdin[0], stdout_pipe[1],
                   stdout_pipe[1] if merge_stderr else stderr_pipe[1], deadline)
        for fd in (owner[0], result[1], stdin[0], stdout_pipe[1], stderr_pipe[1]):
            _release(fd, owned)
        stdout, stderr = Capture(stdout_limit, tail), Capture(stderr_limit, tail)
        code, reason = _pump(stdout_pipe[0], stderr_pipe[0], result[0], stdin[1], input,
                             stdout, stderr, on_stdout, owned)
        if reason == 3:
            raise OSError("could not start the command supervisor or executable")
        if reason == 4:
            raise RuntimeError("command descendants did not terminate")
        return Result(code, bytes(stdout.data), bytes(stderr.data), stdout.truncated,
                      stderr.truncated, reason == 1)
    finally:
        try:
            for fd in tuple(owned):
                _release(fd, owned)
            if guard:
                os.waitpid(guard, 0)
        finally:
            if previous_term is not None:
                signal.signal(signal.SIGTERM, previous_term)
        if terminating:
            raise SystemExit(128 + signal.SIGTERM)

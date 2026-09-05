from __future__ import annotations

import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
import unittest

SUPPORT = Path(__file__).resolve().parents[1] / "python"
sys.path.insert(0, str(SUPPORT))
from fileblade_process import read_input, run


def wait_until(condition, timeout=3):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if condition():
            return True
        time.sleep(0.01)
    return False


def gone(pid):
    return not Path(f"/proc/{pid}").exists()


class Processes(unittest.TestCase):
    def python(self, source, **kwargs):
        return run([sys.executable, "-c", source], timeout=kwargs.pop("timeout", 2),
                   stdout_limit=kwargs.pop("stdout_limit", 4096), **kwargs)

    def test_output_and_private_input(self):
        result = self.python("import sys; sys.stdout.buffer.write(sys.stdin.buffer.read()); print('error', file=sys.stderr)",
                             input=b"private\x00input\xff")
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stdout, b"private\x00input\xff")
        self.assertEqual(result.stderr, b"error\n")

    def test_input_reader_bounds_and_eof(self):
        with tempfile.TemporaryFile() as source:
            source.write(b"private\x00input\xff")
            source.seek(0)
            self.assertEqual(read_input(source, limit=14, timeout=1), b"private\x00input\xff")
            source.seek(0)
            with self.assertRaisesRegex(ValueError, "byte limit"):
                read_input(source, limit=13, timeout=1)
        reader, writer = os.pipe()
        try:
            with os.fdopen(reader, "rb") as source:
                start = time.monotonic()
                with self.assertRaisesRegex(ValueError, "timed out"):
                    read_input(source, limit=16, timeout=0.05)
                self.assertLess(time.monotonic() - start, 1)
        finally:
            os.close(writer)

    def test_prefix_and_tail_bounds(self):
        source = "import os; os.write(1, b'first' + b'x' * 1000000 + b'last'); os.write(2, b'e' * 1000000)"
        prefix = self.python(source, stdout_limit=8, stderr_limit=4)
        self.assertEqual(prefix.stdout, b"firstxxx")
        self.assertTrue(prefix.stdout_truncated and prefix.stderr_truncated)
        tail = self.python(source, stdout_limit=8, stderr_limit=4, tail=True)
        self.assertEqual(tail.stdout, b"xxxxlast")
        self.assertEqual(tail.stderr, b"eeee")
        self.assertTrue(tail.stdout_truncated and tail.stderr_truncated)

    def test_merge_and_streaming(self):
        chunks = []
        result = self.python("import os; os.write(1, b'one'); os.write(2, b'two')",
                             merge_stderr=True, on_stdout=chunks.append)
        self.assertEqual(result.stdout, b"onetwo")
        self.assertEqual(b"".join(chunks), b"onetwo")
        self.assertEqual(result.stderr, b"")

    def test_timeout_after_output_pipe_closed(self):
        start = time.monotonic()
        result = self.python("import os, time; os.close(1); os.close(2); time.sleep(10)", timeout=0.1)
        self.assertTrue(result.timed_out)
        self.assertEqual(result.returncode, 124)
        self.assertLess(time.monotonic() - start, 1.5)

    def test_unread_input_is_cancelled(self):
        start = time.monotonic()
        result = self.python("import time; time.sleep(10)", timeout=0.1, input=b"x" * 1000000)
        self.assertTrue(result.timed_out)
        self.assertLess(time.monotonic() - start, 1.5)

    def test_owned_children_are_stopped_even_when_leader_exits(self):
        source = """import os, signal, time
r, w = os.pipe()
child = os.fork()
if child == 0:
    signal.signal(signal.SIGTERM, signal.SIG_IGN)
    os.write(w, b'1')
    time.sleep(10)
else:
    os.read(r, 1)
    print(child, flush=True)
"""
        result = self.python(source)
        self.assertEqual(result.returncode, 0)
        self.assertTrue(gone(int(result.stdout)))

    def test_fully_detached_helper_keeps_its_independent_lifetime(self):
        source = """import os, time
r, w = os.pipe()
child = os.fork()
if child == 0:
    os.setsid()
    for fd in (0, 1, 2): os.close(fd)
    os.write(w, b'1')
    os.close(w)
    time.sleep(10)
else:
    os.read(r, 1)
    print(child, flush=True)
"""
        result = self.python(source)
        pid = int(result.stdout)
        try:
            self.assertFalse(gone(pid), "fully detached helper was killed with the completed request")
        finally:
            if not gone(pid):
                os.kill(pid, signal.SIGTERM)
                self.assertTrue(wait_until(lambda: gone(pid)))

    def test_private_detached_helper_cannot_hold_the_output_reader(self):
        source = """import ctypes, os, time
r, w = os.pipe()
child = os.fork()
if child == 0:
    os.setsid()
    assert ctypes.CDLL(None).prctl(4, 0, 0, 0, 0) == 0
    os.write(w, b'1')
    time.sleep(10)
else:
    os.read(r, 1)
    print(child, flush=True)
"""
        started = time.monotonic()
        result = self.python(source)
        pid = int(result.stdout)
        try:
            self.assertLess(time.monotonic() - started, 1.5)
            self.assertTrue(result.stdout_truncated and result.stderr_truncated)
        finally:
            if not gone(pid):
                os.kill(pid, signal.SIGTERM)
                self.assertTrue(wait_until(lambda: gone(pid)))

    def test_caller_sigkill_cleans_command_and_descendant(self):
        self.caller_cancellation(kill=True)

    def test_caller_sigterm_waits_until_descendants_are_reaped(self):
        self.caller_cancellation(kill=False)

    def caller_cancellation(self, kill):
        with tempfile.TemporaryDirectory(prefix="fileblade-command-test-") as directory:
            pidfile = Path(directory) / "pids"
            source = f"""import os, signal, time
r, w = os.pipe()
child = os.fork()
if child == 0:
    signal.signal(signal.SIGTERM, signal.SIG_IGN)
    os.write(w, b'1')
    time.sleep(30)
else:
    os.read(r, 1)
    with open({str(pidfile)!r}, 'w') as output: output.write(str(os.getpid()) + ' ' + str(child))
    time.sleep(30)
"""
            caller = "import sys; sys.path.insert(0, " + repr(str(SUPPORT)) + "); from fileblade_process import run; run(" + repr([sys.executable, "-c", source]) + ", timeout=30, stdout_limit=4096)"
            process = subprocess.Popen([sys.executable, "-B", "-c", caller], stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
            try:
                self.assertTrue(wait_until(pidfile.exists))
                pids = [int(value) for value in pidfile.read_text().split()]
                process.kill() if kill else process.terminate()
                process.wait(timeout=2)
                if kill:
                    self.assertTrue(wait_until(lambda: all(gone(pid) for pid in pids)), pids)
                else:
                    self.assertEqual(process.returncode, 143)
                    self.assertTrue(all(gone(pid) for pid in pids), pids)
            finally:
                if process.poll() is None:
                    process.kill()
                process.wait()
                process.stderr.close()

    def test_callback_failure_cleans_children_and_preserves_caller_fds(self):
        with tempfile.TemporaryDirectory(prefix="fileblade-callback-test-") as directory:
            kept = []
            pids = []
            def consume(chunk):
                pids.append(int(chunk))
                kept.append(open(Path(directory) / "kept", "wb"))
                raise ValueError("consumer refused")
            with self.assertRaisesRegex(ValueError, "consumer refused"):
                self.python("import os, time; print(os.getpid(), flush=True); time.sleep(10)", on_stdout=consume)
            self.assertTrue(all(gone(pid) for pid in pids))
            kept[0].write(b"still owned by caller")
            kept[0].close()

    def test_missing_command_and_nonzero_exit(self):
        with self.assertRaises(OSError):
            run(["/nonexistent-fileblade-command"], timeout=0.1, stdout_limit=16)
        result = self.python("raise SystemExit(7)")
        self.assertEqual(result.returncode, 7)


if __name__ == "__main__":
    unittest.main()

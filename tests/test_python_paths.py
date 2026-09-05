import json
import os
from pathlib import Path
import subprocess
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "python"))

from fileblade_paths import NativePath, display, parse_path, path_text, shell_quote, wire


class PathIdentity(unittest.TestCase):
    def test_every_filename_byte(self):
        for byte in range(1, 256):
            if byte == 47:
                continue
            raw = os.fsdecode(b"/fixture/name-" + bytes([byte]))
            self.assertEqual(os.fsencode(parse_path(path_text(raw))), os.fsencode(raw))

    def test_distinct_spellings(self):
        names = [os.fsdecode(b"\xff.txt"), "\ufffd.txt", "\\xFF.txt"]
        self.assertEqual(len({display(name) for name in names}), 3)
        self.assertEqual(len({path_text("/fixture/" + name) for name in names}), 3)
        self.assertEqual(display("line\n\t\\\x7f"), "line\\n\\t\\\\\\u{7F}")

    def test_explicit_path_fields(self):
        raw = os.fsdecode(b"name-\xff")
        value = {"path": NativePath(raw, "/fixture"), "message": raw, "aliases": [NativePath("/" + raw)]}
        encoded = json.dumps(wire(value), ensure_ascii=False).encode("utf-8")
        decoded = json.loads(encoded)
        self.assertEqual(decoded["path"], "file:///fixture/name-%FF")
        self.assertEqual(decoded["message"], "name-\\xFF")
        self.assertEqual(decoded["aliases"], ["file:///name-%FF"])
        with self.assertRaises(ValueError):
            path_text(raw)

    def test_local_uri_validation(self):
        self.assertEqual(parse_path("FILE://localhost/fixture/%FF"), os.fsdecode(b"/fixture/\xff"))
        for invalid in ("/a\0", "file://evil/tmp/a", "file:a", "file:///a%00", "file:///a%GG",
                        "file:///a?b", "file:///a#b", "file:///a\nb", "file:///a b", "file:///a\\b",
                        "file:///a\x7fb", "file://user@localhost/a", "file://localhost:1/a"):
            with self.subTest(invalid=invalid), self.assertRaises(ValueError):
                parse_path(invalid)

    def test_shell_quote_preserves_bytes(self):
        for value in ("/ordinary/file", "a '$\n\\", os.fsdecode(b"/fixture/\xff \"\n$'\\")):
            raw = subprocess.check_output(["bash", "-c", "printf '%s' " + shell_quote(value)])
            self.assertEqual(raw, os.fsencode(value))


if __name__ == "__main__":
    unittest.main()

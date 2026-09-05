import os
from pathlib import Path
import stat
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "python"))
import fileblade_mutations as mutations


class MutationBoundary(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix="fileblade-mutations-")
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.path = self.root / "config.json"
        self.path.write_bytes(b'{"original":true}')
        environment = patch.dict(os.environ, {"XDG_STATE_HOME":str(self.root / "state")})
        environment.start()
        self.addCleanup(environment.stop)

    def snapshot(self):
        return mutations.Snapshot.read(self.path, 4 * 1024 * 1024)

    def test_existing_mode_and_bytes_are_preserved_until_publication(self):
        self.path.chmod(0o640)
        self.snapshot().write(b'{"updated":true}')
        self.assertEqual(self.path.read_bytes(), b'{"updated":true}')
        self.assertEqual(stat.S_IMODE(self.path.stat().st_mode), 0o640)
        self.assertFalse(list(self.root.glob(".fileblade-*")))

    def test_new_configuration_is_private_and_existing_files_are_not_overwritten(self):
        target = self.root / "new" / "config.json"
        pending = mutations.Snapshot.read(target, 1024)
        pending.write(b"private new configuration")
        self.assertEqual(stat.S_IMODE(target.stat().st_mode), 0o600)
        self.assertEqual(stat.S_IMODE(target.parent.stat().st_mode), 0o700)
        with self.assertRaises(OSError):
            pending.write(b"do not replace the new file")
        self.assertEqual(target.read_bytes(), b"private new configuration")

    def test_same_inode_same_size_edit_is_refused_and_retained(self):
        pending = self.snapshot()
        self.path.write_bytes(b'{"replaced":true}')
        with self.assertRaises(OSError):
            pending.write(b"stale transformation")
        self.assertEqual(self.path.read_bytes(), b'{"replaced":true}')
        self.assertFalse(list(self.root.glob(".fileblade-*")))

    def test_retargeted_symlink_and_replaced_ancestors_are_refused(self):
        other = self.root / "other.json"
        other.write_bytes(self.path.read_bytes())
        logical = self.root / "logical"
        logical.symlink_to(self.path)
        pending = mutations.Snapshot.read(logical, 1024)
        logical.unlink(); logical.symlink_to(other)
        with self.assertRaises(OSError):
            pending.write(b"wrong destination")
        self.assertEqual(other.read_bytes(), b'{"original":true}')
        directory = self.root / "directory"
        directory.mkdir()
        source = directory / "config"
        source.write_bytes(b"old")
        pending = mutations.Snapshot.read(source, 1024)
        directory.rename(self.root / "moved")
        directory.mkdir(); source.write_bytes(b"new")
        with self.assertRaises(OSError):
            pending.write(b"wrong directory")
        self.assertEqual(source.read_bytes(), b"new")
        self.assertEqual((self.root / "moved/config").read_bytes(), b"old")

    def test_native_names_remain_distinct_through_the_private_protocol(self):
        native = self.root / os.fsdecode(b"config-\xff")
        unicode = self.root / "config-\ufffd"
        native.write_bytes(b"native"); unicode.write_bytes(b"unicode")
        mutations.Snapshot.read(native, 1024).write(b"updated native")
        self.assertEqual(native.read_bytes(), b"updated native")
        self.assertEqual(unicode.read_bytes(), b"unicode")

    def test_unlink_checks_the_captured_inode_after_a_replacement(self):
        link = self.root / "linked"
        link.symlink_to(self.path)
        original_mutate = mutations.mutate
        def replace_before_native_check(document):
            link.unlink()
            link.write_bytes(b"replacement is not a symlink")
            original_mutate(document)
        with patch.object(mutations, "mutate", side_effect=replace_before_native_check), self.assertRaises(OSError):
            mutations.unlink(link, self.path)
        self.assertEqual(link.read_bytes(), b"replacement is not a symlink")
        self.assertEqual(self.path.read_bytes(), b'{"original":true}')
        self.assertFalse(list(self.root.glob(".fileblade-*")))

    def test_link_creation_is_exclusive_and_matching_unlink_preserves_the_source(self):
        link = self.root / "links" / "link"
        mutations.link(link, self.path)
        self.assertEqual(link.resolve(), self.path)
        with self.assertRaises(OSError):
            mutations.link(link, self.path)
        mutations.unlink(link, self.path)
        self.assertFalse(link.exists())
        self.assertTrue(self.path.exists())

    def test_fifo_and_oversize_are_refused_before_any_source_change(self):
        fifo = self.root / "fifo"
        os.mkfifo(fifo)
        with self.assertRaises(OSError):
            mutations.Snapshot.read(fifo, 1024)
        pending = self.snapshot()
        with self.assertRaises(OSError):
            pending.write(b"x" * (mutations.MAX_CONTENT + 1))
        self.assertEqual(self.path.read_bytes(), b'{"original":true}')

    def test_large_existing_config_and_audit_do_not_leak_private_content(self):
        body = b"private fixture secret:" + b"x" * (2 * 1024 * 1024)
        self.path.write_bytes(body)
        self.snapshot().write(body + b"updated")
        self.assertEqual(self.path.read_bytes(), body + b"updated")
        audit = (self.root / "state/omarchy/fileblade/audit.jsonl").read_text()
        self.assertIn("companion-mutate", audit)
        self.assertNotIn("private fixture secret", audit)
        self.assertNotIn(str(self.path), audit)


if __name__ == "__main__":
    unittest.main()

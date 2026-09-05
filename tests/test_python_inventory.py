import os
import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "python"))
from fileblade_inventory import MAX_WATCH_BYTES, MAX_WATCH_PATHS, WatchPlan, watch_path
from fileblade_paths import parse_path, wire


class InventoryWatchTests(unittest.TestCase):
    def test_missing_directory_tracks_its_existing_parent_then_the_new_directory(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            missing = root / "agent" / "rules"
            with WatchPlan() as plan:
                watch_path(missing, directory=True)
            self.assertEqual(plan.paths, {root})
            missing.mkdir(parents=True)
            with WatchPlan() as plan:
                watch_path(missing, directory=True)
            self.assertEqual(plan.paths, {missing, missing.parent})

    def test_symlink_target_and_alias_parent_both_refresh(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            target = root / "elsewhere"
            target.mkdir()
            source = target / "instructions.md"
            source.touch()
            alias = root / "AGENTS.md"
            alias.symlink_to(source)
            with WatchPlan() as plan:
                watch_path(alias)
            self.assertEqual(plan.paths, {root, target})

    def test_native_bytes_survive_wire_and_do_not_collide(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            paths = {root / os.fsdecode(b"raw-\xff"), root / "raw-\ufffd"}
            for path in paths:
                path.mkdir()
            with WatchPlan() as plan:
                for path in paths:
                    watch_path(path, directory=True)
            document = wire(plan.finish({}))
            self.assertEqual({Path(parse_path(path)) for path in document["watchPaths"]}, paths | {root})

    def test_plan_is_bounded_and_nested_scans_do_not_leak(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with WatchPlan() as outer:
                watch_path(root, directory=True)
                with WatchPlan() as inner:
                    for number in range(MAX_WATCH_PATHS + 1):
                        child = root / str(number)
                        child.mkdir()
                        watch_path(child, directory=True)
                self.assertEqual(len(inner.paths), MAX_WATCH_PATHS)
                self.assertTrue(inner.truncated)
                self.assertEqual(outer.paths, {root, root.parent})
            watch_path(root / "not-in-a-scan", directory=True)
            self.assertEqual(outer.paths, {root, root.parent})

    def test_escaped_watch_paths_fit_the_wire_byte_budget(self):
        plan = WatchPlan()
        plan.paths = {Path("/" + "é" * 1000 + str(index)) for index in range(512)}
        document = wire(plan.finish({}))
        self.assertTrue(document["watchTruncated"])
        self.assertLessEqual(len(json.dumps(document["watchPaths"], separators=(",", ":"))), MAX_WATCH_BYTES)


if __name__ == "__main__":
    unittest.main()

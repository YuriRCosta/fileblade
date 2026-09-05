import QtQuick
import QtTest
import "../../lib/DragPlan.js" as DragPlan

TestCase {
  name: "DragPlanRegression"

  function test_paths_are_snapshotted_without_duplicates_or_nested_sources() {
    compare(DragPlan.disjointPaths(["/work/a", "/work/a/file", "/work/b", "/work/a/"]), ["/work/a", "/work/b"])
    compare(DragPlan.disjointPaths(["/work/a/file", "/work/a"]), ["/work/a"])
    compare(DragPlan.disjointPaths(["/work/a", "/work/ab"]), ["/work/a", "/work/ab"])
  }

  function test_entry_snapshots_follow_the_disjoint_path_set() {
    var entries = [{ path: "/work/a/file" }, { path: "/work/a" }, { path: "/work/b" }]
    compare(DragPlan.entriesForPaths(entries, DragPlan.disjointPaths(entries.map(function(entry) { return entry.path }))),
            [{ path: "/work/a" }, { path: "/work/b" }])
  }

  function test_drop_rejects_self_and_descendants_but_accepts_other_folders() {
    verify(!DragPlan.canDrop([], "/work/to"))
    verify(!DragPlan.canDrop(["/work/from"], "/work/from"))
    verify(!DragPlan.canDrop(["/work/from"], "/work/from/nested"))
    verify(DragPlan.canDrop(["/work/from"], "/work/to"))
  }
}

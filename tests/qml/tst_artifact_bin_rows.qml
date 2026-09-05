import QtQuick
import QtTest
import "../../lib/ArtifactBinRows.js" as ArtifactBinRows

TestCase {
  name: "ArtifactBinRows"

  function live() { return [{ id: "one", name: "one" }, { id: "two", name: "two" }, { id: "three", name: "three" }] }
  function binned(source, position) {
    return { id: "bin:" + source, sourceId: source, name: source, kind: "bin", originKind: "skill", originScope: "user",
             scope: "user", badges: ["binned"], deletedAt: "2026-09-05", position: position }
  }
  function ids(rows) { return rows.map(function(row) { return row.id }) }

  function test_binned_row_replaces_its_live_row_in_place() {
    var rows = ArtifactBinRows.merge(live(), [binned("two", 7)], {})
    compare(ids(rows), ["one", "bin:two", "three"])
    compare(rows[1].kind, "bin")
  }

  function test_binned_row_without_live_match_lands_at_its_position() {
    var rows = ArtifactBinRows.merge([{ id: "one" }, { id: "three" }], [binned("two", 1)], {})
    compare(ids(rows), ["one", "bin:two", "three"])
    var tail = ArtifactBinRows.merge([{ id: "one" }], [{ id: "bin:x", sourceId: "x", kind: "bin" }], {})
    compare(ids(tail), ["one", "bin:x"])
  }

  function test_restore_placeholder_fills_the_gap_until_live_rows_return() {
    var ghost = { row: ArtifactBinRows.placeholderFor(binned("two", 1)), position: 1 }
    var during = ArtifactBinRows.merge([{ id: "one" }, { id: "three" }], [], { two: ghost })
    compare(ids(during), ["one", "two", "three"])
    compare(during[1].kind, "skill")
    compare(during[1].badges.length, 0)
    compare(during[1].deletedAt, "")
    var after = ArtifactBinRows.merge(live(), [], { two: ghost })
    compare(ids(after), ["one", "two", "three"])
    compare(after[1].kind, undefined)
    var rebinned = ArtifactBinRows.merge([{ id: "one" }, { id: "three" }], [binned("two", 1)], { two: ghost })
    compare(ids(rebinned), ["one", "bin:two", "three"])
  }

  function test_replacing_row_inherits_the_live_group_and_index_when_its_record_has_none() {
    var legacy = { id: "bin:two", sourceId: "two", name: "two", kind: "bin", groups: [], position: null }
    var groupsFor = function(entry) { return ["User", entry.name + "-group"] }
    var rows = ArtifactBinRows.merge(live(), [legacy], {}, groupsFor)
    compare(ids(rows), ["one", "bin:two", "three"])
    compare(rows[1].groups, ["User", "two-group"])
    compare(rows[1].position, 1)
    compare(legacy.groups, [])
    compare(legacy.position, null)
    var recorded = binned("two", 7)
    recorded.groups = ["User", "Recorded"]
    var kept = ArtifactBinRows.merge(live(), [recorded], {}, groupsFor)
    compare(kept[1].groups, ["User", "Recorded"])
    compare(kept[1].position, 7)
    var unmatched = ArtifactBinRows.merge([{ id: "one" }], [legacy], {}, groupsFor)
    compare(ids(unmatched), ["one", "bin:two"])
    compare(unmatched[1].groups, [])
  }

  function test_source_of_prefers_the_original_id() {
    compare(ArtifactBinRows.sourceOf(binned("two", 0)), "two")
    compare(ArtifactBinRows.sourceOf({ id: "plain" }), "plain")
    compare(ArtifactBinRows.sourceOf(null), "")
  }
}

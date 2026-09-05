import QtQuick
import QtTest
import "../../lib/KeyedRows.js" as KeyedRows

TestCase {
  name: "KeyedRows"
  property var model: null
  Component { id: modelComponent; ListModel {} }

  function keyOf(row) { return row.key }
  function fieldsOf(row) { return { text: row.text } }
  function keys() { var out = []; for (var i = 0; i < model.count; i++) out.push(model.get(i).rowKey); return out }
  function init() { model = createTemporaryObject(modelComponent, this) }
  function cleanup() { if (model) model.destroy(); model = null }

  function test_initial_fill_and_stable_resync() {
    KeyedRows.sync(model, [{ key: "a", text: "A" }, { key: "b", text: "B" }], keyOf, fieldsOf)
    compare(keys(), ["a", "b"])
    compare(model.get(1).text, "B")
    KeyedRows.sync(model, [{ key: "a", text: "A2" }, { key: "b", text: "B" }], keyOf, fieldsOf)
    compare(keys(), ["a", "b"])
    compare(model.get(0).text, "A2")
  }

  function test_insert_remove_and_move_keep_surviving_rows() {
    KeyedRows.sync(model, [{ key: "a" }, { key: "b" }, { key: "c" }], keyOf, fieldsOf)
    KeyedRows.sync(model, [{ key: "c" }, { key: "x" }, { key: "a" }], keyOf, fieldsOf)
    compare(keys(), ["c", "x", "a"])
    KeyedRows.sync(model, [], keyOf, fieldsOf)
    compare(model.count, 0)
  }

  function paths() { var out = []; for (var i = 0; i < model.count; i++) out.push(model.get(i).path); return out }
  function seed(rows) { model.clear(); for (var i = 0; i < rows.length; i++) model.append(rows[i]) }

  function test_segment_sync_updates_survivors_in_place() {
    seed([{ path: "/r", size: 0 }, { path: "/r/a", size: 1 }, { path: "/r/b", size: 2 }, { path: "/tail", size: 9 }])
    var removed = 0
    model.rowsRemoved.connect(function() { removed++ })
    var stats = KeyedRows.syncSegment(model, 1, 2, [{ path: "/r/a", size: 5 }, { path: "/r/b", size: 2 }], "path")
    compare(paths(), ["/r", "/r/a", "/r/b", "/tail"])
    compare(model.get(1).size, 5)
    compare(stats.updated, 1)
    compare(stats.inserted, 0)
    compare(stats.removed, 0)
    compare(stats.moved, 0)
    verify(!stats.structural)
    compare(removed, 0)
  }

  function test_segment_sync_inserts_removes_and_moves_inside_the_segment_only() {
    seed([{ path: "/r", size: 0 }, { path: "/r/a", size: 1 }, { path: "/r/b", size: 2 }, { path: "/r/c", size: 3 }, { path: "/tail", size: 9 }])
    var stats = KeyedRows.syncSegment(model, 1, 3, [{ path: "/r/c", size: 3 }, { path: "/r/new", size: 4 }, { path: "/r/a", size: 1 }], "path")
    compare(paths(), ["/r", "/r/c", "/r/new", "/r/a", "/tail"])
    compare(stats.inserted, 1)
    compare(stats.removed, 1)
    compare(stats.moved, 1)
    compare(stats.updated, 0)
    verify(stats.structural)
    compare(model.get(4).size, 9)
  }

  function test_segment_sync_empties_and_fills_a_segment() {
    seed([{ path: "/r", size: 0 }, { path: "/r/a", size: 1 }, { path: "/tail", size: 9 }])
    KeyedRows.syncSegment(model, 1, 1, [], "path")
    compare(paths(), ["/r", "/tail"])
    var stats = KeyedRows.syncSegment(model, 1, 0, [{ path: "/r/x", size: 1 }, { path: "/r/y", size: 2 }], "path")
    compare(paths(), ["/r", "/r/x", "/r/y", "/tail"])
    compare(stats.inserted, 2)
  }

  function test_duplicate_keys_get_distinct_suffixes() {
    KeyedRows.sync(model, [{ key: "a" }, { key: "a" }, { key: "a" }], keyOf, fieldsOf)
    compare(keys(), ["a", "a#1", "a#2"])
    KeyedRows.sync(model, [{ key: "a" }, { key: "a" }], keyOf, fieldsOf)
    compare(keys(), ["a", "a#1"])
  }
}

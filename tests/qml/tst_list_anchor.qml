import QtQuick
import QtTest
import "../../ui"

TestCase {
  id: suite
  name: "ListAnchor"
  visible: true
  width: 200
  height: 200

  property bool treeLoading: false
  property var view: null

  ListModel { id: model }

  Component {
    id: viewComponent
    ListView {
      width: 120
      height: 50
      clip: true
      boundsBehavior: Flickable.StopAtBounds
      delegate: Item { width: 120; height: 10 }
    }
  }

  ListAnchor {
    id: anchor
    view: suite.view
    loading: treeLoading
    indexOf: function(path) { for (var i = 0; i < model.count; i++) if (model.get(i).path === path) return i; return -1 }
    keyAt: function(index) { return index >= 0 && index < model.count ? model.get(index).path : "" }
    parentOf: function(path) { var cut = path.lastIndexOf("/"); return cut > 0 ? path.slice(0, cut) : "" }
  }

  function fill(paths) {
    model.clear()
    for (var i = 0; i < paths.length; i++) model.append({ path: paths[i] })
  }

  function tree(count) {
    var paths = ["/root"]
    for (var i = 0; i < count; i++) paths.push("/root/dir/file-" + i)
    return paths
  }

  function settle() { waitForRendering(view); view.forceLayout() }
  function firstVisible() {
    var index = view.indexAt(1, view.contentY + 1)
    var item = index >= 0 ? view.itemAtIndex(index) : null
    return { index: index, offset: item ? view.contentY - item.y : -1 }
  }
  function expectFirst(index, offset) {
    var seen = firstVisible()
    compare(seen.index, index)
    compare(seen.offset, offset)
  }

  function init() {
    treeLoading = false
    anchor.clear()
    model.clear()
    view = createTemporaryObject(viewComponent, this, { model: model })
    verify(view !== null)
    fill(tree(30))
    settle()
    view.contentY = 73
    settle()
    expectFirst(7, 3)
  }

  function cleanup() {
    if (view) view.destroy()
    view = null
  }

  function test_capture_records_the_first_visible_row_and_its_offset() {
    verify(anchor.capture())
    compare(anchor.key, "/root/dir/file-6")
    compare(anchor.offset, 3)
    verify(anchor.pending)
  }

  function test_restore_returns_to_the_same_row_after_a_full_rebuild() {
    expectFirst(7, 3)
    verify(anchor.capture())
    model.clear()
    settle()
    fill(tree(30))
    settle()
    verify(firstVisible().index !== 7 || firstVisible().offset !== 3)
    verify(anchor.restore())
    expectFirst(7, 3)
    verify(!anchor.pending)
  }

  function test_restore_survives_rows_inserted_above() {
    verify(anchor.capture())
    var paths = tree(30)
    paths.splice(1, 0, "/root/above-a", "/root/above-b", "/root/above-c")
    fill(paths)
    settle()
    verify(anchor.restore())
    expectFirst(10, 3)
  }

  function test_restore_waits_while_loading_then_keeps_the_anchor() {
    verify(anchor.capture())
    treeLoading = true
    fill(["/root"])
    settle()
    verify(!anchor.restore())
    verify(anchor.pending)
    fill(tree(30))
    settle()
    verify(anchor.restore())
    expectFirst(7, 3)
    verify(anchor.pending)
    treeLoading = false
    verify(anchor.restore())
    verify(!anchor.pending)
  }

  function test_missing_row_falls_back_to_the_nearest_ancestor_once_loaded() {
    fill(["/root", "/root/dir", "/root/dir/a", "/root/dir/b", "/root/dir/c", "/root/dir/d", "/root/dir/e", "/root/dir/f", "/root/dir/g", "/root/dir/h", "/root/tail-1", "/root/tail-2", "/root/tail-3", "/root/tail-4", "/root/tail-5", "/root/tail-6", "/root/tail-7", "/root/tail-8"])
    settle()
    view.contentY = 62
    settle()
    verify(anchor.capture())
    compare(anchor.key, "/root/dir/e")
    fill(["/root", "/root/dir", "/root/tail-1", "/root/tail-2", "/root/tail-3", "/root/tail-4", "/root/tail-5", "/root/tail-6", "/root/tail-7", "/root/tail-8"])
    settle()
    verify(anchor.restore())
    expectFirst(1, 0)
    verify(!anchor.pending)
  }

  function test_gone_without_any_ancestor_clears_quietly() {
    verify(anchor.capture())
    fill(["/elsewhere/one", "/elsewhere/two"])
    settle()
    verify(!anchor.restore())
    verify(!anchor.pending)
  }

  function test_clear_drops_a_pending_anchor() {
    verify(anchor.capture())
    anchor.clear()
    verify(!anchor.pending)
    verify(!anchor.restore())
  }
}

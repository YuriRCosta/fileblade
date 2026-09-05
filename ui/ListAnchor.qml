import QtQuick

QtObject {
  id: anchor

  property var view: null
  property var indexOf: function(key) { return -1 }
  property var keyAt: function(index) { return "" }
  property var parentOf: function(key) { return "" }
  property bool loading: false
  property string key: ""
  property real offset: 0
  readonly property bool pending: key !== ""

  function capture() {
    clear()
    if (!view || view.count <= 0 || view.height <= 0) return false
    var y = Number(view.contentY) || 0
    var index = view.indexAt(1, y + 1)
    var item = index >= 0 ? view.itemAtIndex(index) : null
    var found = index >= 0 ? String(keyAt(index) || "") : ""
    if (!item || !found) return false
    key = found
    offset = Math.max(0, y - item.y)
    return true
  }

  function fallbackIndex() {
    var candidate = String(parentOf(key) || "")
    var seen = {}
    while (candidate && !seen[candidate]) {
      var index = indexOf(candidate)
      if (index >= 0) return index
      seen[candidate] = true
      candidate = String(parentOf(candidate) || "")
    }
    return -1
  }

  function restore() {
    if (!key || !view) return false
    var index = indexOf(key)
    var exact = index >= 0
    if (!exact) {
      if (loading) return false
      index = fallbackIndex()
    }
    if (index < 0) {
      clear()
      return false
    }
    view.positionViewAtIndex(index, ListView.Beginning)
    var top = Number(view.originY) || 0
    var bottom = top + Math.max(0, Number(view.contentHeight) - Number(view.height))
    view.contentY = Math.max(top, Math.min(bottom, Number(view.contentY) + (exact ? offset : 0)))
    if (!loading || !exact) clear()
    return true
  }

  function clear() {
    key = ""
    offset = 0
  }
}

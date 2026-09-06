.pragma library

function snapshot(tabs) {
  var list = Array.isArray(tabs) ? tabs : []
  var rows = []
  for (var i = 0; i < list.length; i++) {
    var entry = list[i] || {}
    rows.push({ module: String(entry.module || ""), state: entry.state === undefined ? null : entry.state })
  }
  return JSON.stringify(rows)
}

function capture(tabs, index, slotId) {
  var list = Array.isArray(tabs) ? tabs : []
  var position = Number(index)
  if (!isFinite(position) || position < 0 || position >= list.length) return null
  return { index: position, slotId: String(slotId || ""), snapshot: snapshot(list) }
}

function matches(tabs, slotId, captured) {
  if (!captured) return false
  if (String(slotId || "") !== captured.slotId) return false
  var list = Array.isArray(tabs) ? tabs : []
  if (captured.index >= list.length) return false
  return snapshot(list) === captured.snapshot
}

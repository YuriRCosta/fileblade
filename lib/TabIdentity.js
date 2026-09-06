.pragma library

function capture(tabs, index) {
  var list = Array.isArray(tabs) ? tabs : []
  var position = Number(index)
  if (!isFinite(position) || position < 0 || position >= list.length) return null
  var entry = list[position] || {}
  return { index: position, module: String(entry.module || ""), count: list.length }
}

function matches(tabs, captured) {
  if (!captured) return false
  var list = Array.isArray(tabs) ? tabs : []
  if (list.length !== captured.count || captured.index >= list.length) return false
  var entry = list[captured.index] || {}
  return String(entry.module || "") === captured.module
}

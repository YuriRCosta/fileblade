.pragma library

function sourceOf(entry) {
  return String(entry && (entry.sourceId || entry.id) || "")
}

function placeholderFor(entry) {
  var row = {}
  for (var key in entry) row[key] = entry[key]
  row.id = sourceOf(entry)
  row.kind = String(entry.originKind || "")
  row.scope = String(entry.originScope || entry.scope || "")
  row.badges = []
  row.deletedAt = ""
  return row
}

function inheriting(row, liveRow, groupsFor, position) {
  var hasGroups = Array.isArray(row.groups) && row.groups.length > 0
  var hasPosition = row.position !== undefined && row.position !== null && row.position !== ""
  if (hasGroups && hasPosition) return row
  var out = {}
  for (var key in row) out[key] = row[key]
  if (!hasGroups && typeof groupsFor === "function") {
    var groups = groupsFor(liveRow)
    if (Array.isArray(groups) && groups.length > 0) out.groups = groups.slice()
  }
  if (!hasPosition) out.position = position
  return out
}

function merge(live, cached, restoring, groupsFor) {
  var result = Array.isArray(live) ? live.slice() : []
  var index = {}
  for (var l = 0; l < result.length; l++) {
    var liveId = String(result[l] && result[l].id || "")
    if (liveId !== "" && index[liveId] === undefined) index[liveId] = l
  }
  var placed = []
  var trailing = []
  var binned = {}
  var rows = Array.isArray(cached) ? cached : []
  for (var i = 0; i < rows.length; i++) {
    if (!rows[i]) continue
    var source = String(rows[i].sourceId || "")
    if (source !== "") binned[source] = true
    if (source !== "" && index[source] !== undefined) {
      result[index[source]] = inheriting(rows[i], result[index[source]], groupsFor, index[source])
      continue
    }
    var rawPosition = rows[i].position
    var position = Number(rawPosition)
    if (rawPosition !== undefined && rawPosition !== null && rawPosition !== "" && isFinite(position) && position >= 0)
      placed.push({ row: rows[i], position: Math.floor(position), order: i })
    else trailing.push(rows[i])
  }
  var pending = restoring || {}
  var order = rows.length
  for (var id in pending) {
    if (index[id] !== undefined || binned[id]) continue
    placed.push({ row: pending[id].row, position: pending[id].position, order: order++ })
  }
  placed.sort(function(left, right) { return left.position - right.position || left.order - right.order })
  for (var j = 0; j < placed.length; j++)
    result.splice(Math.min(placed[j].position, result.length), 0, placed[j].row)
  return result.concat(trailing)
}

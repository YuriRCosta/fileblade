.pragma library

var RANKS = { D: 4, U: 4, M: 3, T: 3, A: 2, "?": 2, R: 1, C: 1 }

function rank(status) {
  return RANKS[String(status || "")] || 0
}

function defaultStatus(row) {
  if (!row || row.gitIgnored) return ""
  return String(row.gitStatus || "")
}

function rowCount(source) {
  if (!source) return 0
  if (Array.isArray(source)) return source.length
  return Number(source.count) || 0
}

function rowAt(source, index) {
  return Array.isArray(source) ? source[index] : source.get(index)
}

function viewRange(contentY, originY, height, contentHeight) {
  var total = Number(contentHeight) || 0
  if (total <= 0) return { start: 0, end: 1 }
  var start = ((Number(contentY) || 0) - (Number(originY) || 0)) / total
  var end = start + (Number(height) || 0) / total
  return { start: Math.max(0, Math.min(1, start)), end: Math.max(0, Math.min(1, end)) }
}

function inView(fraction, range) {
  return fraction >= range.start && fraction <= range.end
}

function collect(source, slots, statusOf) {
  var count = rowCount(source)
  var buckets = Math.floor(Number(slots) || 0)
  var status = typeof statusOf === "function" ? statusOf : defaultStatus
  if (count <= 0 || buckets <= 0) return []
  var best = {}
  for (var index = 0; index < count; index++) {
    var value = String(status(rowAt(source, index)) || "")
    var weight = rank(value)
    if (weight === 0) continue
    var slot = Math.min(buckets - 1, Math.floor(index * buckets / count))
    var existing = best[slot]
    if (!existing || existing.rank < weight) best[slot] = { rank: weight, status: value, index: index }
  }
  var marks = []
  for (var key in best) marks.push({ fraction: (best[key].index + 0.5) / count, status: best[key].status, index: best[key].index })
  marks.sort(function(left, right) { return left.index - right.index })
  return marks
}

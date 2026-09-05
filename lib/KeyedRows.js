.pragma library

function keysFor(rows, keyOf) {
  var counts = {}
  var keys = []
  for (var i = 0; i < rows.length; i++) {
    var base = String(keyOf(rows[i]))
    var seen = counts[base] || 0
    counts[base] = seen + 1
    keys.push(seen === 0 ? base : base + "#" + seen)
  }
  return keys
}

function sync(model, rows, keyOf, fieldsOf) {
  var keys = keysFor(rows, keyOf)
  var wanted = {}
  for (var w = 0; w < keys.length; w++) wanted[keys[w]] = true
  for (var m = model.count - 1; m >= 0; m--)
    if (!wanted[model.get(m).rowKey]) model.remove(m)
  for (var i = 0; i < rows.length; i++) {
    var fields = fieldsOf(rows[i])
    fields.rowKey = keys[i]
    if (i < model.count && model.get(i).rowKey === keys[i]) {
      model.set(i, fields)
      continue
    }
    var found = -1
    for (var n = i + 1; n < model.count; n++)
      if (model.get(n).rowKey === keys[i]) { found = n; break }
    if (found >= 0) {
      model.move(found, i, 1)
      model.set(i, fields)
    } else {
      model.insert(i, fields)
    }
  }
  while (model.count > rows.length) model.remove(model.count - 1)
  return keys
}

function updateRow(model, index, row) {
  var current = model.get(index)
  var changed = 0
  for (var field in row) {
    if (current[field] === row[field]) continue
    model.setProperty(index, field, row[field])
    changed++
  }
  return changed
}

function syncSegment(model, start, count, rows, keyField) {
  var end = start + count
  var wanted = {}
  for (var w = 0; w < rows.length; w++) wanted[String(rows[w][keyField])] = true
  var stats = { inserted: 0, removed: 0, moved: 0, updated: 0 }
  for (var m = end - 1; m >= start; m--) {
    if (wanted[String(model.get(m)[keyField])]) continue
    model.remove(m)
    end--
    stats.removed++
  }
  for (var i = 0; i < rows.length; i++) {
    var target = start + i
    var key = String(rows[i][keyField])
    if (target < end && String(model.get(target)[keyField]) === key) {
      stats.updated += updateRow(model, target, rows[i])
      continue
    }
    var found = -1
    for (var n = target + 1; n < end; n++)
      if (String(model.get(n)[keyField]) === key) { found = n; break }
    if (found >= 0) {
      model.move(found, target, 1)
      stats.moved++
      stats.updated += updateRow(model, target, rows[i])
    } else {
      model.insert(target, rows[i])
      end++
      stats.inserted++
    }
  }
  stats.structural = stats.inserted + stats.removed + stats.moved > 0
  return stats
}

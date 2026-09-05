.pragma library
.import "PathText.js" as PathText

function cleanPath(value) {
  return value ? PathText.fromFileUrl(String(value)) : ""
}

function containsPath(parent, child) {
  var root = cleanPath(parent)
  var path = cleanPath(child)
  if (!root || !path) return false
  return path !== root && PathText.within(path, root)
}

function disjointPaths(values) {
  var raw = Array.isArray(values) ? values : []
  var unique = []
  for (var i = 0; i < raw.length; i++) {
    var path = cleanPath(raw[i])
    if (path && unique.indexOf(path) < 0) unique.push(path)
  }
  return unique.filter(function(path) {
    for (var index = 0; index < unique.length; index++)
      if (unique[index] !== path && containsPath(unique[index], path)) return false
    return true
  })
}

function entriesForPaths(entries, paths) {
  var rows = Array.isArray(entries) ? entries : []
  var wanted = Array.isArray(paths) ? paths : []
  return rows.filter(function(entry) {
    return !!entry && wanted.indexOf(cleanPath(entry.path)) >= 0
  })
}

function canDrop(paths, destination) {
  var sources = disjointPaths(paths)
  var target = cleanPath(destination)
  if (!target || sources.length === 0) return false
  for (var i = 0; i < sources.length; i++)
    if (sources[i] === target || containsPath(sources[i], target)) return false
  return true
}

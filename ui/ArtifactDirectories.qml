import QtQuick
import "../lib/PathText.js" as PathText

Item {
  id: directories
  visible: false

  property var files: null
  property var paths: []
  property bool active: true
  property int maximumDirectories: 256
  property int entryLimit: 1000
  readonly property bool showHidden: files ? files.showHidden === true : false
  property var wanted: []
  property var cache: ({})
  property var failures: ({})
  property var queue: []
  property var request: null
  property var watch: null
  property int generation: 0
  property string watchError: ""
  property bool limited: false
  readonly property string error: watchError || (limited ? "Too many expanded folders; collapse some to load more"
    : Object.keys(failures).map(function(path) { return failures[path] }).filter(Boolean).join("; ").slice(0, 200))

  function children(path) { return cache[path] || [] }

  function rowFor(raw, parentPath) {
    if (!raw || !PathText.fileUrl(raw.path) || PathText.fileUrl(PathText.parent(raw.path)) !== PathText.fileUrl(parentPath)) return null
    var row = files.makeRow(raw, 0)
    row.linkTarget = String(raw.link_target || "")
    row.detail = row.kind
    row.agents = []
    row.metrics = ({})
    return row
  }

  function cancelRead() {
    var previous = request
    request = null
    if (previous) previous.files.cancelBackendRequest(previous.id, previous.generation, true)
  }

  function stopWatch() {
    var previous = watch
    watch = null
    watchError = ""
    if (previous) previous.files.cancelBackendRequest(previous.id, previous.generation, true)
  }

  function reconcile() {
    var next = [], seen = ({})
    if (active && files) for (var i = 0; i < paths.length; i++) {
      var path = String(paths[i]), key = PathText.fileUrl(path)
      if (key && !seen[key]) { next.push(path); seen[key] = true }
    }
    limited = next.length > maximumDirectories
    next = next.slice(0, maximumDirectories).sort()
    if (JSON.stringify(next) === JSON.stringify(wanted)) return
    wanted = next
    cancelRead()
    var kept = ({}), errors = ({})
    for (var path of wanted) {
      if (cache[path] !== undefined) kept[path] = cache[path]
      if (failures[path]) errors[path] = failures[path]
    }
    cache = kept
    failures = errors
    queue = wanted.filter(function(path) { return cache[path] === undefined })
    stopWatch()
    startWatch()
    debounce.restart()
  }

  function refresh(retryWatch) {
    if (!active || !files) return
    cancelRead()
    queue = wanted.slice()
    if (retryWatch === true || !watch) { stopWatch(); startWatch() }
    debounce.restart()
  }

  function startRead() {
    if (!active || !files || request || queue.length === 0) return
    var batch = queue.slice(0, 8)
    queue = queue.slice(batch.length)
    var args = []
    for (var path of batch) args.push("--path", path)
    args.push("--limit", String(entryLimit), "--sort", "name")
    if (showHidden) args.push("--show-hidden")
    var pending = { id: "", generation: ++generation, files: files, paths: batch }
    request = pending
    pending.id = files.backendRequest("children-batch", args, pending.generation, function(response) {
      if (directories.request !== pending) return
      directories.request = null
      var stored = Object.assign({}, directories.cache), errors = Object.assign({}, directories.failures)
      var results = response && Array.isArray(response.results) ? response.results : []
      for (var path of pending.paths) {
        var result = results.find(function(value) { return value && PathText.fileUrl(value.path) === PathText.fileUrl(path) })
        var rows = []
        errors[path] = ""
        if (!result || result.ok === false || !Array.isArray(result.entries))
          errors[path] = String(result && result.error || response && response.error || "Folder could not be read").slice(0, 200)
        else {
          for (var raw of result.entries.slice(0, directories.entryLimit)) {
            var row = directories.rowFor(raw, path)
            if (row) rows.push(row)
          }
          if (result.truncated === true || result.entries.length > directories.entryLimit)
            errors[path] = "Folder contents capped; open it in Files to see more"
        }
        stored[path] = rows
      }
      directories.cache = stored
      directories.failures = errors
      debounce.restart()
    })
  }

  function startWatch() {
    if (!active || !files || watch || wanted.length === 0) return
    var pending = { id: "", generation: ++generation, files: files }
    watch = pending
    pending.id = files.backendSubscribe(wanted.slice(), pending.generation, function(event) {
      if (directories.watch !== pending) return
      if (event && (event.overflow || (event.events || []).some(function(name) {
        return name === "delete_self" || name === "move_self" || name === "unmount" || name === "ignored"
      }))) directories.stopWatch()
      directories.refresh()
    }, function(response) {
      if (directories.watch !== pending) return
      if (response && Array.isArray(response.skipped) && response.skipped.length)
        directories.watchError = "Some folders could not be watched; refresh to retry"
      directories.refresh()
    }, function(response) {
      if (directories.watch !== pending) return
      directories.watch = null
      if (!response || !response.cancelled) directories.watchError = "Folder watch stopped; refresh to retry"
    })
  }

  onPathsChanged: Qt.callLater(reconcile)
  onActiveChanged: Qt.callLater(reconcile)
  onShowHiddenChanged: refresh()
  onFilesChanged: {
    cancelRead()
    stopWatch()
    wanted = []
    cache = ({})
    failures = ({})
    queue = []
    Qt.callLater(reconcile)
  }
  Component.onCompleted: Qt.callLater(reconcile)
  Component.onDestruction: { cancelRead(); stopWatch() }
  Timer { id: debounce; interval: 120; onTriggered: directories.startRead() }
}

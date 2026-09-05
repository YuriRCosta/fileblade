import QtQuick

Item {
  id: controller

  required property var service
  property alias model: favoritesModel
  property bool busy: false
  property string error: ""
  property int generation: 0
  property string activeRequestId: ""
  property bool queued: false

  ListModel { id: favoritesModel }

  function normalizeRoot(path) { return service.normalizeRoot(path) }

  function indexOfPath(path) {
    var target = normalizeRoot(path)
    for (var rowIndex = 0; rowIndex < favoritesModel.count; rowIndex++)
      if (String(favoritesModel.get(rowIndex).path) === target) return rowIndex
    return -1
  }

  function fallbackEntry(favorite) {
    return {
      name: favorite.name,
      path: favorite.path,
      is_dir: !!favorite.isDir,
      is_symlink: !!favorite.isSymlink,
      is_git_repo: !!favorite.isGitRepo,
      mime: favorite.mime,
      kind: favorite.isDir ? "Directory" : "File"
    }
  }

  function carriedGitFields(previous) {
    if (!previous) return {}
    return {
      isGitRepo: !!previous.isGitRepo,
      gitDeleted: !!previous.gitDeleted,
      gitIgnored: !!previous.gitIgnored,
      gitRepoRoot: String(previous.gitRepoRoot || ""),
      gitStatus: String(previous.gitStatus || ""),
      gitStatusLabel: String(previous.gitStatusLabel || ""),
      gitIndexStatus: String(previous.gitIndexStatus || ""),
      gitWorktreeStatus: String(previous.gitWorktreeStatus || ""),
      gitOriginalPath: String(previous.gitOriginalPath || ""),
      gitModifiedCount: Number(previous.gitModifiedCount) || 0,
      gitDeletedCount: Number(previous.gitDeletedCount) || 0,
      gitNewCount: Number(previous.gitNewCount) || 0,
      gitUntrackedCount: Number(previous.gitUntrackedCount) || 0,
      gitSummary: String(previous.gitSummary || ""),
      gitRepoName: String(previous.gitRepoName || ""),
      gitBranch: String(previous.gitBranch || ""),
      gitWorktree: String(previous.gitWorktree || "")
    }
  }

  function rebuild(entriesByPath, errorsByPath) {
    var favorites = Array.isArray(service.favorites) ? service.favorites : []
    var previous = ({})
    for (var oldIndex = 0; oldIndex < favoritesModel.count; oldIndex++) {
      var oldRow = favoritesModel.get(oldIndex)
      previous[String(oldRow.path)] = oldRow
    }
    favoritesModel.clear()
    for (var index = 0; index < favorites.length; index++) {
      var favorite = favorites[index]
      var path = normalizeRoot(favorite.path)
      var entry = entriesByPath[path]
      var row = service.makeRow(entry || fallbackEntry(favorite), 0)
      if (service.gitEnabled && entry && !entry.git_repo_root) Object.assign(row, carriedGitFields(previous[path]))
      row.error = entry ? "" : String(errorsByPath[path] || "")
      favoritesModel.append(row)
    }
  }

  function refresh() {
    if (!service.stateReady) return
    var favorites = Array.isArray(service.favorites) ? service.favorites : []
    if (favorites.length === 0) {
      cancel()
      favoritesModel.clear()
      return
    }
    if (activeRequestId) {
      queued = true
      return
    }
    var arguments = []
    for (var index = 0; index < favorites.length; index++) arguments.push("--path", String(favorites[index].path))
    busy = true
    generation++
    var requestGeneration = generation
    activeRequestId = service.backendRequest("stat-batch", arguments, requestGeneration, function(response) {
      if (requestGeneration !== controller.generation) return
      controller.activeRequestId = ""
      controller.finish(response || { ok: false, error: "Favorites stat request failed", entries: [], errors: [] })
    })
  }

  function cancel() {
    if (!activeRequestId) return
    service.cancelBackendRequest(activeRequestId, generation)
    generation++
    activeRequestId = ""
    busy = false
    queued = false
  }

  function finish(response) {
    var entriesByPath = ({})
    var entries = Array.isArray(response.entries) ? response.entries : []
    for (var entryIndex = 0; entryIndex < entries.length; entryIndex++)
      entriesByPath[normalizeRoot(entries[entryIndex].path)] = entries[entryIndex]
    var errorsByPath = ({})
    var errors = Array.isArray(response.errors) ? response.errors : []
    for (var errorIndex = 0; errorIndex < errors.length; errorIndex++)
      errorsByPath[normalizeRoot(errors[errorIndex].path)] = String(errors[errorIndex].error || "Unavailable")
    rebuild(entriesByPath, errorsByPath)
    error = String(response.error || "")
    busy = false
    service.scheduleGitMetadataRefresh("favorites")
    if (queued) {
      queued = false
      Qt.callLater(controller.refresh)
    }
  }

  Connections {
    target: service
    function onFavoritesChanged() { controller.refresh() }
    function onStateReadyChanged() { controller.refresh() }
    function onRootPathChanged() { controller.refresh() }
  }
}

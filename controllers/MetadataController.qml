import QtQuick

Item {
  id: controller

  required property var service
  property var selected: null
  property string fingerprint: ""
  property bool busy: false
  property string error: ""
  property string activePath: ""
  property string pendingPath: ""
  property var activeResponse: null
  property int cancellationCount: 0
  property string activeRequestId: ""
  property int generation: 0

  function lookupPending(path) {
    return (activeRequestId && activePath === path) || pendingPath === path
  }

  function reset() {
    selected = null
    fingerprint = ""
    busy = false
    error = ""
    pendingPath = ""
    if (activeRequestId) {
      cancellationCount++
      service.cancelBackendRequest(activeRequestId, generation)
      generation++
      activeRequestId = ""
    }
  }

  function request(path) {
    if (!path) return
    if (activeRequestId) {
      cancellationCount++
      service.cancelBackendRequest(activeRequestId, generation)
      generation++
      activeRequestId = ""
    }
    activePath = path
    activeResponse = null
    busy = true
    generation++
    var requestGeneration = generation
    activeRequestId = service.backendRequest("stat", ["--path", path], requestGeneration, function(response) {
      if (requestGeneration !== controller.generation) return
      controller.activeRequestId = ""
      controller.finish(response)
    })
  }

  function finish(response) {
    response = response || { ok: false, error: "Metadata request failed" }
    if (activePath === service.selectedPath) apply(response)
    var next = pendingPath
    activePath = ""
    activeResponse = null
    pendingPath = ""
    if (next && next !== service.selectedPath) next = service.selectedPath
    if (next) Qt.callLater(function() { controller.request(next) })
  }

  function apply(response) {
    if (response.ok && response.entry) {
      var summary = service.gitEnabled ? (selected || ({})) : ({})
      selected = Object.assign({}, response.entry, {
        is_deleted: !!summary.is_deleted,
        git_repo_root: String(summary.git_repo_root || ""),
        git_status: String(summary.git_status || ""),
        git_status_label: String(summary.git_status_label || ""),
        git_index_status: String(summary.git_index_status || ""),
        git_worktree_status: String(summary.git_worktree_status || ""),
        git_original_path: String(summary.git_original_path || ""),
        git_modified_count: Math.max(0, Number(summary.git_modified_count) || 0),
        git_deleted_count: Math.max(0, Number(summary.git_deleted_count) || 0),
        git_new_count: Math.max(0, Number(summary.git_new_count) || 0),
        git_untracked_count: Math.max(0, Number(summary.git_untracked_count) || 0),
        git_summary: summary.git_summary || null,
        git_repo_name: String(summary.git_repo_name || ""),
        git_branch: String(summary.git_branch || ""),
        git_worktree: String(summary.git_worktree || "")
      })
      fingerprint = String(response.entry.stat_fingerprint || fingerprint)
      error = ""
    } else {
      error = String(response.error || "Unable to read properties")
    }
    busy = false
  }

}

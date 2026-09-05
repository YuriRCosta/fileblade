import QtQuick

Item {
  id: controller

  required property var service
  property alias model: applicationModel
  property string mime: ""
  property string path: ""
  property bool busy: false
  property string error: ""
  property bool loaded: false
  property string activePath: ""
  property string pendingPath: ""
  property string pendingMime: ""
  property int lookupCount: 0
  property int cancellationCount: 0
  property int cacheHitCount: 0
  property int mimeHintCount: 0
  property string activeRequestId: ""
  property int generation: 0

  ListModel { id: applicationModel }

  function mimeHint(targetPath) {
    var target = String(targetPath || "")
    var entry = service.selectedMetadata && String(service.selectedMetadata.path || "") === target
      ? service.selectedMetadata
      : service.entryForKnownPath(target)
    if (!entry || entry.permissions === undefined) return ""
    var value = String(entry.mime || "").trim().toLowerCase()
    return value && value !== "application/octet-stream" && value.indexOf("/") > 0 ? value : ""
  }

  function start(targetPath, hint) {
    var target = String(targetPath || "")
    if (!target) {
      busy = false
      return
    }
    activePath = target
    busy = true
    lookupCount++
    var arguments = ["--path", target]
    if (hint) {
      arguments.push("--mime", hint)
      mimeHintCount++
    }
    generation++
    var requestGeneration = generation
    activeRequestId = service.backendRequest("applications", arguments, requestGeneration, function(result) {
      if (requestGeneration !== controller.generation) return
      controller.activeRequestId = ""
      controller.finish(result)
    })
  }

  function cancel() {
    pendingPath = ""
    pendingMime = ""
    path = ""
    busy = false
    if (activeRequestId) {
      cancellationCount++
      service.cancelBackendRequest(activeRequestId, generation)
      generation++
      activeRequestId = ""
    }
  }

  function load(targetPath) {
    var target = String(targetPath || service.selectedPath)
    path = target
    error = ""
    if (!target) return
    var hint = mimeHint(target)
    if (!activeRequestId && loaded && hint && hint === mime) {
      cacheHitCount++
      busy = false
      return
    }
    if (activeRequestId && activePath === target) return
    applicationModel.clear()
    mime = ""
    loaded = false
    busy = true
    if (activeRequestId) {
      cancellationCount++
      service.cancelBackendRequest(activeRequestId, generation)
      generation++
      activeRequestId = ""
    }
    start(target, hint)
  }

  function finish(result) {
    result = result || { ok: false, error: "Application lookup failed" }
    var current = activePath !== "" && activePath === path
      && service.actionMenuOpen && service.actionMenuMode === "open-with"
    if (current) apply(result)
    var nextPath = pendingPath
    var nextMime = pendingMime
    activePath = ""
    pendingPath = ""
    pendingMime = ""
    if (nextPath && nextPath === path && service.actionMenuOpen && service.actionMenuMode === "open-with")
      Qt.callLater(function() { controller.start(nextPath, nextMime) })
    else if (!current)
      busy = false
  }

  function apply(result) {
    applicationModel.clear()
    mime = String(result.mime || "")
    var applications = Array.isArray(result.applications) ? result.applications : []
    for (var i = 0; i < applications.length; i++) applicationModel.append(applications[i])
    error = result.ok ? "" : String(result.error || "Application lookup failed")
    loaded = !!result.ok
    busy = false
  }

}

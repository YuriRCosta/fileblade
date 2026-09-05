import QtQuick

Item {
  id: controller

  required property var service

  readonly property string anchor: String(service.selectedPath || service.rootPath || "")
  readonly property string folderContextPath: service.selectedPath && service.selectedMetadata
    && String(service.selectedMetadata.path || "") === service.selectedPath && service.selectedMetadata.is_dir
    ? service.selectedPath : String(service.rootPath || "")
  readonly property bool projectContextActive: service.projectContext && projectMarker === ".git"
    && projectRoot !== "" && resolvedAnchor === anchor
  readonly property string contextPath: projectContextActive ? projectRoot : folderContextPath
  property string projectRoot: ""
  property string projectName: ""
  property string projectMarker: ""
  property bool projectInferred: true
  property string requested: ""
  property bool busy: false
  property int generation: 0
  property string resolvedAnchor: ""

  function refresh() {
    if (!service.pluginDir) return
    if (anchor.indexOf("://") >= 0) {
      generation++
      requested = ""
      busy = false
      projectRoot = ""
      projectName = ""
      projectMarker = ""
      projectInferred = true
      resolvedAnchor = ""
      return
    }
    if (busy) {
      requested = anchor
      return
    }
    requested = ""
    busy = true
    generation++
    var requestGeneration = generation
    var requestAnchor = anchor
    service.backendRequest("project-root", ["--path", requestAnchor], requestGeneration, function(response) {
      if (requestGeneration !== controller.generation) return
      controller.busy = false
      controller.apply(response, requestAnchor)
      var rerun = controller.requested
      controller.requested = ""
      if (rerun || requestAnchor !== controller.anchor) Qt.callLater(controller.refresh)
    })
  }

  function apply(parsed, requestAnchor) {
    if (!parsed || !parsed.ok) return
    projectRoot = String(parsed.root || "")
    projectName = String(parsed.name || "")
    projectMarker = String(parsed.marker || "")
    projectInferred = parsed.inferred === true
    resolvedAnchor = String(requestAnchor || "")
  }

  onAnchorChanged: debounce.restart()
  Component.onCompleted: refresh()

  Timer {
    id: debounce
    interval: 120
    onTriggered: controller.refresh()
  }

}

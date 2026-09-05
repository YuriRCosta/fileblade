import QtQuick
import "../modules/welcome/WelcomePlan.js" as WelcomePlan

Item {
  id: controller
  visible: false
  required property var service

  readonly property string state: String(service.welcomeState || "")
  readonly property bool pending: WelcomePlan.pending(state)
  readonly property var extensions: WelcomePlan.EXTENSIONS
  property var queue: []
  property int installed: 0
  property bool installing: false
  property string error: ""
  property int generation: 0
  property bool checking: false
  property var intended: []
  property var placing: []
  property bool placingActive: false
  property bool placementFailed: false
  property bool awaitingSave: false
  property int placementTimeout: 30000
  readonly property bool busy: installing || placingActive
  readonly property bool ready: service.stateReady && service.backendReady
    && !!service.bladeHost && service.bladeHost.layoutReady

  onReadyChanged: {
    if (!ready) return
    if (pending) checkProgress()
    else removeTabs()
  }
  onPendingChanged: Qt.callLater(reconcileTabs)

  Connections {
    target: service.bladeHost
    function onLayoutChanged() { Qt.callLater(controller.reconcileTabs) }
    function onLayoutWritableChanged() { controller.placePending() }
    function onLastWrittenLayoutTextChanged() { controller.settleSave() }
  }

  function reconcileTabs() {
    if (ready && !pending) removeTabs()
    if (placingActive) placePending()
  }

  Connections {
    target: service.bladeHost ? service.bladeHost.registry : null
    function onRegistryChanged() { Qt.callLater(controller.placePending) }
  }

  Timer {
    id: placementDeadline
    interval: controller.placementTimeout
    onTriggered: controller.abandonPlacement()
  }

  Timer {
    id: progressTimer
    interval: 1000
    onTriggered: controller.checkProgress()
  }

  function registryHas(moduleId) {
    var host = service.bladeHost
    return !!(host && host.registry && host.registry.module(moduleId))
  }

  function missing() {
    return WelcomePlan.missing(registryHas)
  }

  function removeTabs() {
    var host = service.bladeHost
    if (!host) return
    for (var attempt = 0; attempt < 64; attempt++) {
      var found = host.findModule("welcome")
      if (!found || !host.removeTab(found.edge, found.index, found.tab)) return
    }
  }

  function finish(next) {
    service.setWelcomeState(next)
    removeTabs()
  }

  function beginPlacement() {
    var host = service.bladeHost
    if (!host || !host.layoutReady || busy) return
    error = ""
    placementFailed = false
    intended = WelcomePlan.unplaced(function(module) { return !!host.findModule(module) }).map(function(placement) { return placement.module })
    placing = intended
    placingActive = true
    placementDeadline.restart()
    placePending()
  }

  function placePending() {
    var host = service.bladeHost
    if (!host || !placingActive || !host.layoutReady) return
    var rest = []
    for (var i = 0; i < WelcomePlan.PLACEMENTS.length; i++) {
      var placement = WelcomePlan.PLACEMENTS[i]
      if (intended.indexOf(placement.module) < 0 || host.findModule(placement.module)) continue
      if (!host.layoutWritable || !registryHas(placement.module) || !placeOne(host, placement) || !host.findModule(placement.module)) rest.push(placement.module)
    }
    placing = rest
    if (rest.length === 0) completePlacement(host)
    else awaitingSave = false
  }

  function completePlacement(host) {
    var slots = host.slots(WelcomePlan.PLACEMENT_EDGE)
    var top = WelcomePlan.topSlotIndex(slots)
    if (top >= 0 && Number(slots[top].active) !== 0) host.setSlotTab(WelcomePlan.PLACEMENT_EDGE, top, 0)
    awaitingSave = true
    settleSave()
  }

  function layoutSaved(host) {
    if (!host.layoutWritable) return false
    try {
      return JSON.stringify(JSON.parse(String(host.lastWrittenLayoutText || ""))) === JSON.stringify(host.layoutDocument())
    } catch (error) {
      return false
    }
  }

  function settleSave() {
    var host = service.bladeHost
    if (!awaitingSave || !host || !layoutSaved(host)) return
    awaitingSave = false
    placementDeadline.stop()
    placingActive = false
    finish("installed")
  }

  function abandonPlacement() {
    error = awaitingSave
      ? "The blade layout was not saved; click Install to retry"
      : "Could not add " + placing.map(WelcomePlan.extensionName).join(", ") + " to the right blade; click Install to retry"
    awaitingSave = false
    placing = []
    intended = []
    placingActive = false
    placementFailed = true
  }

  function placeOne(host, placement) {
    var edge = WelcomePlan.PLACEMENT_EDGE
    if (placement.target === "top") {
      var top = WelcomePlan.topSlotIndex(host.slots(edge))
      return top >= 0 ? host.addTab(edge, top, placement.module, {}) : host.addSlot(edge, placement.module, 0)
    }
    var notes = host.findModule("notes")
    if (notes && notes.edge === edge) return host.addTab(edge, notes.index, placement.module, {})
    return host.addSlot(edge, placement.module, -1)
  }

  function dismiss() {
    if (busy) return false
    placing = []
    intended = []
    finish("dismissed")
    return true
  }

  function install() {
    if (busy) return false
    error = ""
    installed = 0
    queue = missing()
    if (queue.length === 0) {
      beginPlacement()
      return true
    }
    installing = true
    queue = extensions
    service.startExtensionInstall()
    progressTimer.restart()
    return true
  }

  function checkProgress() {
    if (!ready || checking) return
    checking = true
    var request = { generation: ++generation }
    service.backendRequest("plugin-install-status", [], request.generation, function(response) {
      if (request.generation !== controller.generation) return
      controller.checking = false
      if (!response || response.ok !== true) {
        controller.error = String(response && (response.message || response.error) || "Could not read installation progress")
        controller.installing = false
        return
      }
      controller.installed = Number(response.installed || 0)
      controller.queue = controller.extensions
      controller.installing = response.state === "running"
      if (controller.installing) {
        progressTimer.restart()
      } else if (response.state === "installed") {
        controller.beginPlacement()
      } else if (response.state === "failed") {
        controller.error = String(response.message || "Installation stopped; click Install to retry")
      }
    })
  }
}

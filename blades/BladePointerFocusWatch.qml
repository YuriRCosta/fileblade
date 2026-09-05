import QtQuick

Item {
  id: watch

  required property var controller
  readonly property var service: controller.service
  readonly property string focusedEdge: controller.focusedEdge
  readonly property int focusRevision: controller.focusRevision
  readonly property bool watching: focusedEdge !== ""
    && !controller.isWindowMode(focusedEdge) && !!service
  property bool baselineReady: false
  property real baselineX: 0
  property real baselineY: 0
  property int stillTicks: 0
  property string requestId: ""
  property int generation: 0

  function begin() {
    stop()
    if (!watching) return
    baselineReady = false
    stillTicks = 0
    pollTimer.interval = 0
    pollTimer.restart()
  }

  function stop() {
    pollTimer.stop()
    if (requestId && service) service.cancelBackendRequest(requestId, generation)
    generation++
    requestId = ""
    baselineReady = false
  }

  function scheduleNext() {
    if (!watching) return
    pollTimer.interval = Math.min(480, 80 * (1 + Math.floor(stillTicks / 8)))
    pollTimer.restart()
  }

  function poll() {
    if (!watching || requestId) return
    var edge = focusedEdge
    var arguments = [
      "--blade-title", controller.windowTitle("left"),
      "--blade-title", controller.windowTitle("right")
    ]
    generation++
    var requestGeneration = generation
    requestId = service.backendRequest("hover-target", arguments, requestGeneration, function(parsed) {
      if (requestGeneration !== watch.generation) return
      watch.requestId = ""
      if (!watch.watching || watch.focusedEdge !== edge || !parsed || parsed.ok === false) {
        watch.scheduleNext()
        return
      }
      var x = Number(parsed.x)
      var y = Number(parsed.y)
      if (!isFinite(x) || !isFinite(y)) {
        watch.scheduleNext()
        return
      }
      if (!watch.baselineReady) {
        watch.baselineReady = true
        watch.baselineX = x
        watch.baselineY = y
        watch.scheduleNext()
        return
      }
      var moved = Math.abs(x - watch.baselineX) + Math.abs(y - watch.baselineY) >= 2
      if (!moved) {
        watch.stillTicks++
        watch.scheduleNext()
        return
      }
      watch.baselineX = x
      watch.baselineY = y
      watch.stillTicks = 0
      if (parsed.action !== "hover-focus" || watch.controller.pointerBusy) {
        watch.scheduleNext()
        return
      }
      watch.controller.releaseFocus(edge)
      watch.controller.restoreFocusAddress = String(parsed.address || "")
      watch.controller.restoreFocusClass = ""
      watch.controller.restoreWorkspaceFocus()
    }, null, 2000)
  }

  onFocusedEdgeChanged: begin()
  onFocusRevisionChanged: begin()
  onWatchingChanged: begin()
  Component.onCompleted: begin()
  Component.onDestruction: stop()

  Timer {
    id: pollTimer
    repeat: false
    onTriggered: watch.poll()
  }
}

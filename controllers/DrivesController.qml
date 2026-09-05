import QtQuick

Item {
  id: controller

  required property var service
  property alias model: drivesModel
  property alias allModel: allVolumesModel
  property bool running: false
  property bool actionsAvailable: false
  property string error: ""
  property string busySource: ""
  property int generation: 0
  property string requestId: ""
  property int failureCount: 0
  property bool showSystemVolumes: false

  ListModel { id: drivesModel }
  ListModel { id: allVolumesModel }

  readonly property int count: drivesModel.count
  readonly property int volumeCount: allVolumesModel.count
  readonly property var tierOrder: ["external", "unmounted", "internal", "system"]

  function tierLabel(tier) {
    if (tier === "external") return "External"
    if (tier === "unmounted") return "Not mounted"
    if (tier === "internal") return "Internal"
    if (tier === "system") return "System"
    return String(tier || "")
  }

  function tierRank(tier) {
    var rank = tierOrder.indexOf(String(tier || ""))
    return rank < 0 ? tierOrder.length : rank
  }

  function restartDelay() {
    return Math.min(30000, 500 * Math.pow(2, Math.min(failureCount, 6)))
  }

  function visibleTier(tier) {
    if (tier === "external" || tier === "unmounted") return true
    return showSystemVolumes
  }

  function glyphFor(volume) {
    if (volume.image) return "󰗮"
    if (volume.bus === "usb") return "󱊞"
    if (volume.removable) return "󰑹"
    return "󰋊"
  }

  function actionFor(row) {
    if (!row.mounted) {
      var context = []
      if (row.readOnly) context.push({ text: "mounts read-only" })
      if (row.needsAuthorization) context.push({ text: "asks for authentication" })
      return { command: "mount-volume", glyph: "󰄠", tip: "Mount " + row.name, actions: [{ button: "left", text: "Mount" }], context: context }
    }
    if (row.external) return { command: "eject-volume", glyph: "󰇪", tip: "Eject " + row.name, actions: [{ button: "left", text: "Eject" }], context: [] }
    return { command: "unmount-volume", glyph: "󰇪", tip: "Unmount " + row.name, actions: [{ button: "left", text: "Unmount" }], context: [] }
  }

  function rowFor(volume) {
    var size = Number(volume.size) || 0
    var used = volume.used === null || volume.used === undefined ? -1 : Number(volume.used)
    return {
      name: String(volume.name || volume.device || ""),
      device: String(volume.device || ""),
      source: String(volume.source || ""),
      mountpoint: String(volume.mountpoint || ""),
      mounted: !!volume.mounted,
      filesystem: String(volume.filesystem || ""),
      label: String(volume.label || ""),
      bus: String(volume.bus || ""),
      image: String(volume.image || ""),
      size: size,
      sizeLabel: String(volume.size_label || ""),
      used: used,
      available: Number(volume.available) || 0,
      usedFraction: size > 0 && used >= 0 ? used / size : -1,
      removable: !!volume.removable,
      external: !!volume.external,
      readOnly: !!volume.read_only,
      needsAuthorization: !!volume.needs_authorization,
      tier: String(volume.tier || ""),
      volumeGlyph: glyphFor(volume)
    }
  }

  function apply(payload) {
    if (!payload || !Array.isArray(payload.volumes)) return
    actionsAvailable = !!payload.actions
    var rows = []
    for (var index = 0; index < payload.volumes.length; index++) rows.push(rowFor(payload.volumes[index]))
    rows.sort(function(left, right) {
      var byTier = tierRank(left.tier) - tierRank(right.tier)
      return byTier !== 0 ? byTier : left.name.localeCompare(right.name)
    })
    drivesModel.clear()
    allVolumesModel.clear()
    for (var rowIndex = 0; rowIndex < rows.length; rowIndex++) {
      allVolumesModel.append(rows[rowIndex])
      if (visibleTier(rows[rowIndex].tier)) drivesModel.append(rows[rowIndex])
    }
  }

  function start() {
    if (!service.backendReady || requestId) return
    generation++
    var requestGeneration = generation
    running = true
    requestId = service.backendSubscribeTopic("mounts", [], requestGeneration, function(event) {
      if (requestGeneration !== controller.generation) return
      controller.apply(event.payload)
    }, function(response) {
      if (requestGeneration !== controller.generation) return
      controller.running = true
      controller.failureCount = 0
      controller.error = ""
      controller.apply(response.payload)
    }, function(response) {
      if (requestGeneration !== controller.generation) return
      controller.requestId = ""
      controller.running = false
      var failed = !(response && (response.ok || response.cancelled))
      if (!failed) return
      controller.failureCount++
      controller.error = String(response && response.error || "drive watcher stopped")
      console.warn("data-goblin.fileblade: drive watcher failed (" + controller.failureCount + "): " + controller.error)
      restartTimer.interval = controller.restartDelay()
      restartTimer.restart()
    })
  }

  function stop() {
    if (!requestId) return
    service.cancelBackendRequest(requestId, generation)
    generation++
    requestId = ""
    running = false
  }

  function indexOfSource(source) {
    for (var index = 0; index < allVolumesModel.count; index++)
      if (String(allVolumesModel.get(index).source) === String(source)) return index
    return -1
  }

  function clearError() { error = "" }

  function act(command, source) {
    if (busySource !== "" || !actionsAvailable) return
    busySource = String(source)
    error = ""
    service.backendRequest(command, ["--source", String(source)], generation, function(response) {
      controller.busySource = ""
      if (response && response.ok) return
      controller.error = String(response && response.error || command + " failed")
    }, null, 200000, { untimed: true })
  }

  function mountVolume(source) { act("mount-volume", source) }
  function unmountVolume(source) { act("unmount-volume", source) }
  function ejectVolume(source) { act("eject-volume", source) }

  function runAction(source) {
    var index = indexOfSource(source)
    if (index < 0) return
    act(actionFor(allVolumesModel.get(index)).command, source)
  }

  function openVolume(source, targetScreen) {
    var index = indexOfSource(source)
    if (index < 0) return
    var row = allVolumesModel.get(index)
    if (!row.mounted) {
      mountVolume(source)
      return
    }
    service.navigateToLocation(String(row.mountpoint), targetScreen, "favorite")
  }

  Timer {
    id: restartTimer
    interval: 500
    onTriggered: controller.start()
  }

  Connections {
    target: service
    function onBackendReadyChanged() {
      if (service.backendReady) controller.start()
      else {
        controller.requestId = ""
        controller.running = false
      }
    }
  }

  Component.onCompleted: start()
}

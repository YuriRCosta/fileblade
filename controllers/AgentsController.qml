import QtQuick

Item {
  id: controller

  required property var service

  property var installedAgents: []
  property var agentDetails: []
  property bool refreshQueued: false
  property bool busy: false
  property int generation: 0

  function refresh() {
    if (!service.pluginDir) return
    if (busy) {
      refreshQueued = true
      return
    }
    busy = true
    generation++
    var requestGeneration = generation
    service.backendRequest("agents", [], requestGeneration, function(response) {
      if (requestGeneration !== controller.generation) return
      controller.busy = false
      controller.apply(response)
      if (controller.refreshQueued) {
        controller.refreshQueued = false
        Qt.callLater(controller.refresh)
      }
    })
  }

  function apply(parsed) {
    if (!parsed || !parsed.ok || !Array.isArray(parsed.agents)) return
    var ids = []
    for (var i = 0; i < parsed.agents.length && i < 32; i++)
      if (parsed.agents[i].installed === true) ids.push(String(parsed.agents[i].id))
    agentDetails = parsed.agents.slice(0, 32)
    installedAgents = ids
  }

  Connections {
    target: controller.service
    function onOpenChanged() { if (controller.service.open) controller.refresh() }
  }

  Component.onCompleted: refresh()
}

import QtQuick

Item {
  id: controller
  required property var service
  property var states: ({})
  property var restoreRoutes: ({})
  property int generation: 0
  property bool stopping: false
  signal finished(string module, string requestId, var response)

  function stateFor(module) { return states[String(module || "")] || { busy: false, error: "", id: "" } }

  function registerRestore(module, route) {
    if (!/^[a-z][a-z0-9-]{0,31}$/.test(module) || !route || !route.provider || !route.directory || !route.helper) return false
    if (!restoreRoutes[module] && Object.keys(restoreRoutes).length >= 256) return false
    var next = Object.assign({}, restoreRoutes)
    next[module] = { provider: String(route.provider), directory: String(route.directory), helper: String(route.helper) }
    restoreRoutes = next
    return true
  }

  function restoreArguments(module) {
    var route = restoreRoutes[String(module || "")]
    return route ? ["--helper-route", JSON.stringify(route)] : []
  }

  function publish(module, state) {
    var next = Object.assign({}, states)
    next[module] = state
    states = next
  }

  function run(module, command, arguments) {
    if (stopping || !/^[a-z][a-z0-9-]{0,31}$/.test(module) || stateFor(module).busy || !Array.isArray(arguments)) return ""
    if (["bin-put", "bin-remove", "bin-restore", "bin-purge", "trash"].indexOf(command) < 0) return ""
    if (!states[module] && Object.keys(states).length >= 256) return ""
    var request = { id: "", busy: true, error: "", generation: ++generation }
    publish(module, request)
    request.id = service.backendRequest(command, arguments.slice(), request.generation, function(response) {
      if (controller.stopping || controller.states[module] !== request) return
      if (response && response.pending_commit)
        response = { ok: false, error: "Open the owning companion to restore this legacy record" }
      var failure = response && (response.error || response.message)
      var error = response && response.ok === true ? "" : String(failure || "Recovery operation failed").slice(0, 400)
      controller.publish(module, { id: request.id, busy: false, error: error })
      if (controller.service.refreshTrash) controller.service.refreshTrash()
      controller.finished(module, request.id, response)
    }, null, 35000, { untimed: true })
    return request.id
  }

  Component.onDestruction: {
    stopping = true
    for (var module in states) {
      var request = states[module]
      if (request.busy && request.id) service.cancelBackendRequest(request.id, request.generation, true)
    }
  }
}

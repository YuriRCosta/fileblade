import QtQuick
import QtTest
import "../../controllers"

TestCase {
  id: test
  name: "ArtifactActions"
  property var requests: []
  property var cancelled: []
  property int refreshes: 0
  property int delivered: 0
  property var actions: null

  Item {
    id: files
    function backendRequest(command, args, generation, callback, progress, deadline, options) {
      var id = "action-" + requests.length
      requests.push({id:id, command:command, args:args, generation:generation, callback:callback, options:options})
      return id
    }
    function cancelBackendRequest(id, generation, discard) { cancelled.push({id:id, generation:generation, discard:discard}) }
    function refreshTrash() { test.refreshes++ }
  }
  Component { id: controller; ArtifactActionController { service: files } }
  Component {
    id: observer
    Item {
      readonly property bool busy: !!actions && actions.stateFor("hooks").busy
      Connections { target: actions; function onFinished(module, requestId, response) { test.delivered++ } }
    }
  }
  function init() {
    requests = []; cancelled = []; refreshes = 0; delivered = 0
    actions = createTemporaryObject(controller, test)
  }
  function cleanup() { if (actions) actions.destroy(); actions = null; wait(0) }

  function test_accepted_action_finishes_after_its_view_is_destroyed() {
    var view = createTemporaryObject(observer, test)
    var args = ["--module", "hooks", "--item", "original"]
    var id = actions.run("hooks", "bin-remove", args)
    args[3] = "changed"
    verify(view.busy); compare(requests[0].args[3], "original")
    verify(requests[0].options.untimed)
    view.destroy(); wait(0)
    compare(cancelled.length, 0)
    requests[0].callback({ok:true})
    compare(actions.stateFor("hooks").id, id)
    verify(!actions.stateFor("hooks").busy)
    compare(delivered, 0); compare(refreshes, 1)
    var reopened = createTemporaryObject(observer, test)
    verify(!reopened.busy)
  }
  function test_duplicate_action_is_refused_and_stale_completion_cannot_settle_a_new_one() {
    actions.run("hooks", "bin-remove", [])
    compare(actions.run("hooks", "bin-remove", []), "")
    requests[0].callback({ok:true})
    actions.run("hooks", "bin-restore", [])
    requests[0].callback({ok:false, error:"old"})
    verify(actions.stateFor("hooks").busy)
    requests[1].callback({ok:false, message:"source changed; recovery kept"})
    compare(actions.stateFor("hooks").error, "source changed; recovery kept")
  }
  function test_routes_are_copied_and_remain_available_after_observer_closure() {
    var route = {provider:"test.hooks",directory:"/plugins/hooks",helper:"inventory"}
    verify(actions.registerRestore("hooks", route))
    route.directory = "/wrong"
    compare(JSON.parse(actions.restoreArguments("hooks")[1]).directory, "/plugins/hooks")
    compare(actions.restoreArguments("unknown"), [])
    verify(!actions.registerRestore("../escape", route))
  }
  function test_core_shutdown_discards_callback_before_context_destruction() {
    actions.run("hooks", "bin-remove", [])
    actions.destroy(); actions = null; wait(0)
    compare(cancelled.length, 1)
    verify(cancelled[0].discard)
    compare(cancelled[0].id, requests[0].id)
  }
}

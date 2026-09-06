import QtQuick
import QtTest
import "../../controllers" as Controllers

TestCase {
  id: testCase
  name: "LocationFocusRegression"
  property var location: null
  property var navigation: null

  QtObject { id: screenA; property string name: "A" }
  QtObject { id: screenB; property string name: "B" }
  QtObject {
    id: fakeHost
    property string focusedMonitorName: "A"
    property var preferred: screenA
    property var focused: []
    function preferredScreen(edge) { return preferred }
    function findModule(module) { return { edge: "left", slot: 0 } }
    function setAllOpen(value) { fakeService.open = value }
    function focusModule(module, screen, part) {
      focused = focused.concat([{ module: module, screen: screen, part: part }])
      return true
    }
  }
  QtObject {
    id: fakeService
    property var bladeHost: fakeHost
    property string rootPath: "/start"
    property bool open: true
    property bool actionMenuOpen: false
    property bool pickerActive: false
    property var treeModel: ({ count: 1 })
    property string trashResource: "trash:"
    property string recentResource: "recent:"
    property string drivesResource: "drives:"
    property var requests: []
    property var canceled: []
    property var focused: []
    property var completions: []
    function normalizeRoot(path) { return String(path) }
    function setRootPath(path) { rootPath = path }
    function preferredScreen() { return fakeHost.preferred }
    function closeActionMenu() { actionMenuOpen = false }
    function cancelLocationValidation() { testCase.location.cancel() }
    function focusTree(screen) { focused = focused.concat([screen]) }
    function locationValidationFinished(screen, success, path, error, monitor) {
      completions = completions.concat([{ screen: screen, success: success, path: path, error: error, monitor: monitor }])
    }
    function backendRequest(verb, arguments, generation, callback) {
      requests = requests.concat([{ verb: verb, arguments: arguments, callback: callback }])
      return "request-" + requests.length
    }
    function cancelBackendRequest(id, generation) { canceled = canceled.concat([id]) }
  }
  Component { id: locationComponent; Controllers.LocationController {} }
  Component { id: navigationComponent; Controllers.NavigationController {} }

  function init() {
    fakeHost.focusedMonitorName = "A"
    fakeHost.preferred = screenA
    fakeHost.focused = []
    fakeService.rootPath = "/start"
    fakeService.open = true
    fakeService.actionMenuOpen = false
    fakeService.requests = []
    fakeService.canceled = []
    fakeService.focused = []
    fakeService.completions = []
    location = locationComponent.createObject(testCase, { service: fakeService })
    navigation = navigationComponent.createObject(testCase, { service: fakeService })
    verify(location)
    verify(navigation)
  }

  function cleanup() {
    location.destroy()
    navigation.destroy()
    location = null
    navigation = null
  }

  function complete(index, path) {
    fakeService.requests[index].callback({ ok: true, entries: [{ path: path, is_dir: true }] })
  }

  function test_browse_finishes_without_focusing_after_monitor_change() {
    location.navigate("/target", screenA, "browse")
    fakeHost.focusedMonitorName = "B"
    complete(0, "/target")
    compare(fakeService.rootPath, "/target")
    compare(fakeService.focused.length, 0)
  }

  function test_browse_keeps_original_target_when_focus_has_not_changed() {
    location.navigate("/target", screenA, "browse")
    fakeHost.preferred = screenB
    complete(0, "/target")
    compare(fakeService.rootPath, "/target")
    compare(fakeService.focused.length, 1)
    compare(fakeService.focused[0], screenA)
  }

  function test_direct_success_retains_completion_owner_and_original_monitor() {
    location.navigate("/target", screenA, "direct")
    fakeHost.focusedMonitorName = "B"
    complete(0, "/target")
    compare(fakeService.rootPath, "/target")
    compare(fakeService.completions.length, 1)
    compare(fakeService.completions[0].screen, screenA)
    compare(fakeService.completions[0].monitor, "A")
    compare(fakeService.completions[0].success, true)
    compare(fakeService.focused.length, 0)
  }

  function test_direct_failure_retains_completion_owner_and_original_monitor() {
    location.navigate("/missing", screenA, "direct")
    fakeHost.focusedMonitorName = "B"
    fakeService.requests[0].callback({ ok: false, error: "Missing" })
    compare(fakeService.rootPath, "/start")
    compare(fakeService.completions.length, 1)
    compare(fakeService.completions[0].screen, screenA)
    compare(fakeService.completions[0].monitor, "A")
    compare(fakeService.completions[0].success, false)
  }

  function test_superseded_backend_answer_cannot_focus_or_replace_latest_navigation() {
    location.navigate("/old", screenA, "browse")
    fakeHost.focusedMonitorName = "B"
    location.navigate("/new", screenB, "browse")
    compare(fakeService.canceled.length, 1)
    complete(0, "/old")
    compare(fakeService.rootPath, "/start")
    compare(fakeService.focused.length, 0)
    complete(1, "/new")
    compare(fakeService.rootPath, "/new")
    compare(fakeService.focused.length, 1)
    compare(fakeService.focused[0], screenB)
  }

  function test_delayed_open_does_not_steal_focus_after_monitor_change() {
    navigation.focusAfterOpen(screenA)
    fakeHost.focusedMonitorName = "B"
    wait(200)
    compare(fakeHost.focused.length, 0)
  }

  function test_delayed_open_retains_explicit_target() {
    navigation.focusAfterOpen(screenA)
    fakeHost.preferred = screenB
    tryCompare(fakeHost, "focused", [{ module: "files", screen: screenA, part: "tree" }])
  }

  function test_delayed_open_captures_an_omitted_target_before_waiting() {
    navigation.focusAfterOpen(null)
    fakeHost.preferred = screenB
    wait(200)
    compare(fakeHost.focused.length, 1)
    compare(fakeHost.focused[0].screen, screenA)
  }

  function test_open_all_schedules_focus_for_its_invocation_monitor() {
    fakeService.open = false
    fakeHost.focusedMonitorName = "B"
    fakeHost.preferred = screenB
    navigation.setOpen(true)
    tryCompare(fakeHost, "focused", [{ module: "files", screen: screenB, part: "tree" }])
  }

  function test_explicit_close_cancels_delayed_open_focus() {
    navigation.focusAfterOpen(screenA)
    navigation.setOpen(false)
    wait(200)
    compare(fakeHost.focused.length, 0)
  }
}

import QtQuick
import QtTest
import "../../controllers" as Controllers

TestCase {
  id: test
  name: "WelcomeLifecycle"

  QtObject {
    id: registry
    signal registryChanged()
    property var known: []
    function module(id) { return known.indexOf(id) >= 0 ? { id: id } : null }
  }

  QtObject {
    id: host
    property bool layoutReady: true
    property var registry: registry
    property var right: [{ modules: [{ module: "welcome" }, { module: "notes" }], active: 0 }]
    property var layout: right.length > 0 ? right[0].modules.map(function(entry) { return entry.module }) : []
    property var activeCalls: []
    property bool layoutWritable: true
    property bool autoSave: true
    property bool refuseAdds: false
    property string lastWrittenLayoutText: ""
    onRightChanged: if (autoSave) lastWrittenLayoutText = JSON.stringify(layoutDocument())
    function layoutDocument() { return { right: right } }
    function flushSave() { lastWrittenLayoutText = JSON.stringify(layoutDocument()) }
    function slots(edge) { return edge === "right" ? right : [] }
    function findModule(name) {
      for (var i = 0; i < right.length; i++) {
        for (var j = 0; j < right[i].modules.length; j++) {
          if (right[i].modules[j].module === name) return { edge: "right", index: i, tab: j }
        }
      }
      return null
    }
    function removeTab(edge, slot, tab) {
      var next = JSON.parse(JSON.stringify(right))
      next[slot].modules.splice(tab, 1)
      right = next
      return true
    }
    function addTab(edge, slot, module, state) {
      if (refuseAdds || edge !== "right" || slot < 0 || slot >= right.length) return false
      var next = JSON.parse(JSON.stringify(right))
      next[slot].modules.push({ module: module })
      next[slot].active = next[slot].modules.length - 1
      right = next
      return true
    }
    function addSlot(edge, module, index) {
      if (refuseAdds || edge !== "right" || !registry.module(module)) return false
      var next = JSON.parse(JSON.stringify(right))
      var slot = { modules: [{ module: module }], active: 0 }
      if (index < 0 || index > next.length) next.push(slot)
      else next.splice(index, 0, slot)
      right = next
      return true
    }
    function setSlotTab(edge, slot, tab) {
      var next = JSON.parse(JSON.stringify(right))
      next[slot].active = tab
      right = next
      activeCalls = activeCalls.concat([[slot, tab]])
      return true
    }
  }

  QtObject {
    id: service
    property bool stateReady: true
    property bool backendReady: true
    property var bladeHost: host
    property string welcomeState: "installed"
    property var installStatus: ({ ok: true, state: "idle" })
    property int installsStarted: 0
    function backendRequest(name, args, generation, callback) {
      callback(installStatus)
    }
    function setWelcomeState(next) { welcomeState = next }
    function startExtensionInstall() { installsStarted++ }
  }

  Controllers.WelcomeController { id: welcome; service: service }

  readonly property var extensionModules: ["data-goblin.fileblade-memory/memory", "data-goblin.fileblade-skills/skills", "data-goblin.fileblade-mcp/mcp", "data-goblin.fileblade-hooks/hooks"]

  function modulesOf(slots) {
    return slots.map(function(slot) { return slot.modules.map(function(entry) { return entry.module }) })
  }

  function init() {
    welcome.placementTimeout = 30000
    welcome.placing = []
    welcome.intended = []
    welcome.placingActive = false
    welcome.awaitingSave = false
    welcome.placementFailed = false
    welcome.error = ""
    service.welcomeState = ""
    service.installStatus = { ok: true, state: "idle" }
    registry.known = []
    host.autoSave = true
    host.refuseAdds = false
    host.layoutWritable = true
    host.right = [{ modules: [{ module: "welcome" }, { module: "notes" }], active: 0 }]
    host.activeCalls = []
    wait(0)
  }

  function reload() {
    service.stateReady = false
    wait(0)
    service.stateReady = true
    wait(0)
  }

  function test_pending_welcome_survives_layout_loading() {
    compare(host.layout, ["welcome", "notes"])
  }

  function test_completion_removes_a_late_restored_welcome() {
    service.welcomeState = "installed"
    tryCompare(host, "layout", ["notes"])
    host.right = [{ modules: [{ module: "welcome" }, { module: "notes" }], active: 0 }]
    tryCompare(host, "layout", ["notes"])
  }

  function test_dismissal_survives_a_late_layout_read() {
    service.welcomeState = "dismissed"
    tryCompare(host, "layout", ["notes"])
    host.right = [{ modules: [{ module: "welcome" }, { module: "notes" }], active: 0 }]
    tryCompare(host, "layout", ["notes"])
  }

  function test_finished_install_places_three_on_top_and_memory_with_notes() {
    var started = service.installsStarted
    welcome.install()
    compare(service.installsStarted, started + 1)
    registry.known = extensionModules
    service.installStatus = { ok: true, state: "installed", installed: 4 }
    welcome.checkProgress()
    tryCompare(service, "welcomeState", "installed")
    compare(modulesOf(host.right), [
      ["data-goblin.fileblade-skills/skills", "data-goblin.fileblade-mcp/mcp", "data-goblin.fileblade-hooks/hooks"],
      ["notes", "data-goblin.fileblade-memory/memory"]
    ])
    compare(host.right[0].active, 0)
    compare(welcome.placing, [])
  }

  function test_placement_waits_for_the_registry_before_recording_completion() {
    registry.known = ["data-goblin.fileblade-skills/skills"]
    welcome.install()
    service.installStatus = { ok: true, state: "installed", installed: 4 }
    welcome.checkProgress()
    wait(0)
    compare(service.welcomeState, "")
    compare(modulesOf(host.right), [["data-goblin.fileblade-skills/skills"], ["welcome", "notes"]])
    compare(welcome.placing.length, 3)
    compare(welcome.placingActive, true)
    registry.known = extensionModules
    registry.registryChanged()
    tryCompare(service, "welcomeState", "installed")
    compare(modulesOf(host.right), [
      ["data-goblin.fileblade-skills/skills", "data-goblin.fileblade-mcp/mcp", "data-goblin.fileblade-hooks/hooks"],
      ["notes", "data-goblin.fileblade-memory/memory"]
    ])
    compare(welcome.placingActive, false)
  }

  function test_reload_after_install_resumes_placement_from_the_status_file() {
    service.installStatus = { ok: true, state: "installed", installed: 4 }
    registry.known = extensionModules
    reload()
    tryCompare(service, "welcomeState", "installed")
    compare(modulesOf(host.right), [
      ["data-goblin.fileblade-skills/skills", "data-goblin.fileblade-mcp/mcp", "data-goblin.fileblade-hooks/hooks"],
      ["notes", "data-goblin.fileblade-memory/memory"]
    ])
  }

  function test_completion_waits_for_the_layout_save() {
    host.autoSave = false
    registry.known = extensionModules
    welcome.install()
    wait(0)
    compare(modulesOf(host.right)[0], ["data-goblin.fileblade-skills/skills", "data-goblin.fileblade-mcp/mcp", "data-goblin.fileblade-hooks/hooks"])
    compare(service.welcomeState, "")
    compare(welcome.awaitingSave, true)
    host.flushSave()
    tryCompare(service, "welcomeState", "installed")
    compare(modulesOf(host.right)[1], ["notes", "data-goblin.fileblade-memory/memory"])
  }

  function test_timeout_names_the_missing_extensions_and_install_retries() {
    welcome.placementTimeout = 40
    registry.known = ["data-goblin.fileblade-skills/skills", "data-goblin.fileblade-mcp/mcp", "data-goblin.fileblade-memory/memory"]
    welcome.install()
    service.installStatus = { ok: true, state: "installed", installed: 4 }
    welcome.checkProgress()
    tryCompare(welcome, "placementFailed", true)
    verify(welcome.error.indexOf("Hooks") >= 0, welcome.error)
    compare(service.welcomeState, "")
    compare(welcome.placingActive, false)
    compare(welcome.dismiss(), true)
    service.welcomeState = ""
    host.right = [
      { modules: [{ module: "data-goblin.fileblade-skills/skills" }, { module: "data-goblin.fileblade-mcp/mcp" }], active: 0 },
      { modules: [{ module: "welcome" }, { module: "notes" }, { module: "data-goblin.fileblade-memory/memory" }], active: 0 }
    ]
    registry.known = extensionModules
    compare(welcome.install(), true)
    tryCompare(service, "welcomeState", "installed")
    compare(modulesOf(host.right), [
      ["data-goblin.fileblade-skills/skills", "data-goblin.fileblade-mcp/mcp", "data-goblin.fileblade-hooks/hooks"],
      ["notes", "data-goblin.fileblade-memory/memory"]
    ])
  }

  function test_stuck_layout_save_is_a_retryable_error_not_a_completion() {
    welcome.placementTimeout = 40
    host.autoSave = false
    registry.known = extensionModules
    welcome.install()
    tryCompare(welcome, "placementFailed", true)
    verify(welcome.error.indexOf("not saved") >= 0, welcome.error)
    compare(service.welcomeState, "")
    compare(welcome.awaitingSave, false)
    host.autoSave = true
    compare(welcome.install(), true)
    wait(0)
    host.flushSave()
    tryCompare(service, "welcomeState", "installed")
  }

  function test_refused_placement_stays_queued_until_it_succeeds() {
    registry.known = extensionModules
    host.refuseAdds = true
    welcome.install()
    wait(0)
    compare(service.welcomeState, "")
    compare(welcome.placing.length, 4)
    host.refuseAdds = false
    registry.registryChanged()
    tryCompare(service, "welcomeState", "installed")
    compare(modulesOf(host.right), [
      ["data-goblin.fileblade-skills/skills", "data-goblin.fileblade-mcp/mcp", "data-goblin.fileblade-hooks/hooks"],
      ["notes", "data-goblin.fileblade-memory/memory"]
    ])
  }

  function test_placement_waits_for_a_writable_layout() {
    registry.known = extensionModules
    host.layoutWritable = false
    welcome.install()
    wait(0)
    compare(modulesOf(host.right), [["welcome", "notes"]])
    compare(welcome.placing.length, 4)
    compare(welcome.placingActive, true)
    host.layoutWritable = true
    tryCompare(service, "welcomeState", "installed")
    compare(modulesOf(host.right), [
      ["data-goblin.fileblade-skills/skills", "data-goblin.fileblade-mcp/mcp", "data-goblin.fileblade-hooks/hooks"],
      ["notes", "data-goblin.fileblade-memory/memory"]
    ])
  }

  function test_a_late_layout_read_that_wipes_placements_is_repaired_before_completion() {
    host.autoSave = false
    registry.known = extensionModules
    welcome.install()
    wait(0)
    compare(welcome.awaitingSave, true)
    host.right = [{ modules: [{ module: "welcome" }, { module: "notes" }], active: 0 }]
    wait(0)
    compare(service.welcomeState, "")
    compare(modulesOf(host.right), [
      ["data-goblin.fileblade-skills/skills", "data-goblin.fileblade-mcp/mcp", "data-goblin.fileblade-hooks/hooks"],
      ["welcome", "notes", "data-goblin.fileblade-memory/memory"]
    ])
    host.flushSave()
    tryCompare(service, "welcomeState", "installed")
    compare(modulesOf(host.right)[1], ["notes", "data-goblin.fileblade-memory/memory"])
  }

  function test_a_late_read_that_changes_the_active_top_tab_is_corrected_before_completion() {
    host.autoSave = false
    registry.known = extensionModules
    welcome.install()
    wait(0)
    compare(host.right[0].active, 0)
    var shifted = JSON.parse(JSON.stringify(host.right))
    shifted[0].active = 2
    host.right = shifted
    wait(0)
    compare(host.right[0].active, 0)
    host.flushSave()
    tryCompare(service, "welcomeState", "installed")
    compare(host.right[0].active, 0)
  }

  function test_dismiss_is_refused_while_tabs_are_being_added() {
    registry.known = []
    welcome.install()
    service.installStatus = { ok: true, state: "installed", installed: 4 }
    welcome.checkProgress()
    wait(0)
    compare(welcome.placingActive, true)
    compare(welcome.dismiss(), false)
    compare(service.welcomeState, "")
  }

  function test_placement_skips_modules_the_user_already_placed() {
    registry.known = extensionModules
    host.right = [
      { modules: [{ module: "data-goblin.fileblade-hooks/hooks" }], active: 0 },
      { modules: [{ module: "welcome" }, { module: "notes" }, { module: "data-goblin.fileblade-memory/memory" }], active: 0 }
    ]
    welcome.install()
    service.installStatus = { ok: true, state: "installed", installed: 4 }
    welcome.checkProgress()
    tryCompare(service, "welcomeState", "installed")
    compare(modulesOf(host.right), [
      ["data-goblin.fileblade-hooks/hooks", "data-goblin.fileblade-skills/skills", "data-goblin.fileblade-mcp/mcp"],
      ["notes", "data-goblin.fileblade-memory/memory"]
    ])
  }

  function test_nothing_missing_still_places_installed_extensions() {
    registry.known = extensionModules
    var started = service.installsStarted
    welcome.install()
    compare(service.installsStarted, started)
    tryCompare(service, "welcomeState", "installed")
    compare(modulesOf(host.right)[0], ["data-goblin.fileblade-skills/skills", "data-goblin.fileblade-mcp/mcp", "data-goblin.fileblade-hooks/hooks"])
  }
}

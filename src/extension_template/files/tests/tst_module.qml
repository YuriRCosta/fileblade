import QtQuick
import QtTest
import "../blades" as Blade

TestCase {
  id: test
  name: "{{PLUGIN_NAME}}Module"
  when: windowShown
  width: 400
  height: 300

  property var calls: []
  property var stored: ({})
  property int attached: 0
  property string selected: "/home/someone/projects/demo/notes.md"

  QtObject {
    id: files
    property string selectedPath: test.selected
    function openDefault(path, screen) { test.calls.push("open:" + path) }
    function openInEditor(path) { test.calls.push("edit:" + path) }
  }

  QtObject {
    id: provider
    function attach(context) { test.attached++ }
    function detach(context) { test.attached-- }
  }

  QtObject {
    id: context
    property bool bladeOpen: true
    property bool collapsed: false
    property var screen: null
    property var providerService: provider
    property QtObject state: QtObject {
      function get(key, fallback) { return key in test.stored ? test.stored[key] : fallback }
      function set(key, value) { test.stored[key] = value; return true }
    }
    property QtObject settings: QtObject {
      function has(key) { return key === "caption" || key === "showSelection" }
      function get(key) { return key === "caption" ? String(test.stored.caption || "") : test.stored.showSelection !== false }
      function set(key, value) { test.stored[key] = value; return true }
    }
    property QtObject paths: QtObject {
      function name(path) { return String(path).split("/").pop() }
    }
    function service(id) { return id === "files" ? files : null }
    function focusNext() { test.calls.push("next") }
    function focusPrevious() { test.calls.push("previous") }
    function closeBlade() { test.calls.push("close") }
    function handlePressed(item, x, y) { test.calls.push("press") }
    function handleMoved(item, x, y) {}
    function handleReleased(item, x, y) { test.calls.push("release") }
    function handleCanceled() {}
  }

  Component {
    id: moduleComponent
    Blade.Module { width: 400; height: 300 }
  }

  function init() {
    test.calls = []
    test.stored = {}
    test.attached = 0
    test.selected = "/home/someone/projects/demo/notes.md"
  }

  function test_title_and_selection() {
    var module = createTemporaryObject(moduleComponent, test, { context: context })
    compare(module.title, "{{PLUGIN_NAME}}")
    compare(module.selectedName, "notes.md")
    compare(module.showSelection, true)
    compare(module.shortcuts.length, 1)
    verify(module.shortcuts[0].items.length >= 4)
  }

  function test_settings_come_from_the_context() {
    test.stored = { caption: "Hello", showSelection: false }
    var module = createTemporaryObject(moduleComponent, test, { context: context })
    compare(module.caption, "Hello")
    compare(module.showSelection, false)
  }

  function test_keys_route_to_the_host() {
    var module = createTemporaryObject(moduleComponent, test, { context: context })
    module.takeFocus("")
    tryCompare(module, "activeFocus", true)
    keyClick(Qt.Key_Tab)
    keyClick(Qt.Key_Backtab)
    keyClick(Qt.Key_Return)
    keyClick(Qt.Key_E)
    keyClick(Qt.Key_Escape)
    compare(test.calls, ["next", "previous", "open:" + test.selected, "edit:" + test.selected, "close"])
  }

  function test_enter_without_a_selection_does_nothing() {
    test.selected = ""
    var module = createTemporaryObject(moduleComponent, test, { context: context })
    compare(module.selectedName, "")
    compare(module.openSelection(false), false)
    compare(test.calls, [])
  }

  function test_provider_attach_and_detach() {
    var module = createTemporaryObject(moduleComponent, test, { context: context })
    compare(test.attached, 1)
    module.destroy()
    wait(0)
    tryCompare(test, "attached", 0)
  }

  function test_loads_without_a_context() {
    var module = createTemporaryObject(moduleComponent, test)
    compare(module.context, null)
    compare(module.selectedName, "")
    compare(module.caption, "")
  }
}

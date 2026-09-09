import QtQuick
import QtTest
import "../../ui" as PluginUi

TestCase {
  id: test
  name: "TrashConfirmation"

  QtObject {
    id: controller
    signal trashConfirmationRequested(var paths)
    property var pendingTrashPaths: []
    property int trashConfirmationSerial: 0
    property var trashed: []
    function request(paths) {
      pendingTrashPaths = paths.slice()
      trashConfirmationSerial++
      trashConfirmationRequested(paths.slice())
    }
    function resolveTrashConfirmation(confirm) {
      var paths = pendingTrashPaths.slice()
      pendingTrashPaths = []
      if (confirm && paths.length > 0) trashed = trashed.concat([paths])
      return paths.length
    }
  }

  component FakeDialog: QtObject {
    property bool opened: false
    property string message: ""
    property int opens: 0
    function open(text, choices) { message = text; opened = true; opens++ }
    function close() { opened = false }
  }

  FakeDialog { id: leftDialog }
  FakeDialog { id: rightDialog }

  PluginUi.TrashConfirmationBinding {
    id: left
    controller: controller
    dialog: leftDialog
    paneVisible: true
    choices: [{ key: "cancel" }, { key: "trash" }]
    promptFor: function(paths) { return "trash " + paths.join(",") }
  }

  PluginUi.TrashConfirmationBinding {
    id: right
    controller: controller
    dialog: rightDialog
    paneVisible: true
    choices: [{ key: "cancel" }, { key: "trash" }]
    promptFor: function(paths) { return "trash " + paths.join(",") }
  }

  function init() {
    controller.pendingTrashPaths = []
    controller.trashed = []
    leftDialog.opened = false
    rightDialog.opened = false
    leftDialog.opens = 0
    rightDialog.opens = 0
    left.paneVisible = true
    right.paneVisible = true
  }

  function test_confirming_on_one_monitor_closes_both_and_trashes_once() {
    controller.request(["/a/bravo.txt"])
    compare(leftDialog.opened, true)
    compare(rightDialog.opened, true)
    compare(rightDialog.message, "trash /a/bravo.txt")
    compare(left.resolve("trash"), true)
    compare(leftDialog.opened, false)
    compare(rightDialog.opened, false)
    compare(controller.trashed, [["/a/bravo.txt"]])
    compare(controller.pendingTrashPaths, [])
  }

  function test_cancel_on_the_other_monitor_closes_both_and_keeps_files() {
    controller.request(["/a/bravo.txt"])
    compare(right.resolve("cancel"), true)
    compare(leftDialog.opened, false)
    compare(rightDialog.opened, false)
    compare(controller.trashed, [])
  }

  function test_hiding_the_prompt_cancels_it_and_a_newer_request_starts_clean() {
    controller.request(["/a/bravo.txt"])
    left.paneVisible = false
    compare(leftDialog.opened, false, "the hidden pane must not keep the bravo prompt")
    compare(controller.pendingTrashPaths, [], "hiding the prompt cancels the bravo request")
    compare(rightDialog.opened, false)
    controller.request(["/a/delta.txt"])
    compare(rightDialog.message, "trash /a/delta.txt")
    compare(leftDialog.opened, false)
    left.paneVisible = true
    compare(leftDialog.opened, false)
    compare(right.resolve("trash"), true)
    compare(controller.trashed, [["/a/delta.txt"]])
  }

  function test_a_stale_prompt_never_answers_a_newer_request() {
    controller.request(["/a/bravo.txt"])
    var stale = left.requestSerial
    leftDialog.opened = true
    left.requestSerial = stale
    controller.trashConfirmationSerial++
    controller.pendingTrashPaths = ["/a/delta.txt"]
    compare(left.resolve("trash"), false)
    compare(leftDialog.opened, false)
    compare(controller.trashed, [])
    compare(controller.pendingTrashPaths, ["/a/delta.txt"])
    compare(left.resolve("cancel"), false)
    compare(controller.pendingTrashPaths, ["/a/delta.txt"])
  }

  function test_hiding_the_pane_that_shows_the_current_prompt_cancels_the_request() {
    right.paneVisible = false
    controller.request(["/a/foxtrot.txt"])
    compare(leftDialog.opened, true)
    left.paneVisible = false
    compare(leftDialog.opened, false)
    compare(controller.pendingTrashPaths, [], "hiding the only visible prompt cancels the request")
    compare(controller.trashed, [])
  }

  function test_hiding_a_pane_with_a_stale_prompt_only_closes_it() {
    controller.request(["/a/golf.txt"])
    var stale = left.requestSerial
    controller.trashConfirmationSerial++
    controller.pendingTrashPaths = ["/a/hotel.txt"]
    leftDialog.opened = true
    left.requestSerial = stale
    left.paneVisible = false
    compare(leftDialog.opened, false)
    compare(controller.pendingTrashPaths, ["/a/hotel.txt"], "a stale prompt never cancels the live request")
  }

  function test_hidden_pane_never_opens_but_visible_pane_does() {
    right.paneVisible = false
    controller.request(["/a/echo.txt"])
    compare(leftDialog.opened, true)
    compare(rightDialog.opened, false)
    compare(rightDialog.opens, 0)
  }
}

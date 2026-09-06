import QtQuick

QtObject {
  id: binding

  required property var controller
  required property var dialog
  property bool paneVisible: true
  property var choices: []
  property var promptFor: function(paths) { return "" }
  property int requestSerial: -1

  readonly property bool current: requestSerial === Number(controller ? controller.trashConfirmationSerial : -2)

  function retire() {
    if (dialog.opened && current && controller.pendingTrashPaths.length > 0) controller.resolveTrashConfirmation(false)
    else if (dialog.opened) dialog.close()
  }

  onPaneVisibleChanged: if (!paneVisible) retire()
  Component.onDestruction: retire()

  function resolve(key) {
    if (!current) {
      if (dialog.opened) dialog.close()
      return false
    }
    controller.resolveTrashConfirmation(key === "trash")
    return true
  }

  readonly property Connections requests: Connections {
    target: binding.controller
    ignoreUnknownSignals: true
    function onTrashConfirmationRequested(paths) {
      if (binding.dialog.opened) binding.dialog.close()
      binding.requestSerial = Number(binding.controller.trashConfirmationSerial)
      if (!binding.paneVisible) return
      binding.dialog.open(binding.promptFor(paths), binding.choices)
    }
    function onPendingTrashPathsChanged() {
      if (binding.dialog.opened && binding.controller.pendingTrashPaths.length === 0) binding.dialog.close()
    }
  }
}

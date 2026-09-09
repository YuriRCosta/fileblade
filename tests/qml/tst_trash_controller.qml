import QtQuick
import QtTest
import "../../controllers"

TestCase {
  name: "TrashControllerRegression"

  property var requests: []

  QtObject {
    id: history
    signal operationCompleted(var response)
    function refreshJournal() {}
  }

  Item {
    id: fakeService
    property bool stateReady: false
    property bool trashCleanupConsent: false
    property int trashRetentionDays: 0
    property double trashLastClearedAt: 0
    property var history: history
    property var pendingCallback: null

    function backendRequest(name, arguments, generation, callback) {
      requests.push({ name: name, arguments: arguments })
      pendingCallback = callback
      return name + "-request"
    }
    function reply(response) {
      var callback = pendingCallback
      pendingCallback = null
      callback(response)
    }
    function backendSubscribe() { return "" }
    function cancelBackendRequest() { return true }
    function artifactTrashHandler() { return null }
    function markTrashCleared() {}
  }

  TrashController {
    id: controller
    service: fakeService
  }

  SignalSpy {
    id: inserted
    target: controller.model
    signalName: "rowsInserted"
  }

  SignalSpy {
    id: removed
    target: controller.model
    signalName: "rowsRemoved"
  }

  SignalSpy {
    id: changed
    target: controller.model
    signalName: "dataChanged"
  }

  function entry(id, name) {
    return {
      id: id,
      name: name,
      original_path: "/home/test/" + name,
      original_parent: "/home/test",
      deleted_at: "2026-09-04T12:34:56",
      size: 4,
      size_text: "4 B",
      kind: "File",
      mime: "text/plain",
      can_restore: true
    }
  }

  function init() {
    fakeService.stateReady = false
    fakeService.trashCleanupConsent = false
    fakeService.trashRetentionDays = 0
    requests = []
    fakeService.pendingCallback = null
    controller.listRequestId = ""
    controller.operationRequestId = ""
    controller.busy = false
    controller.operationBusy = false
    controller.noticeDuration = 3000
    controller.clearNotice()
    controller.presented = []
    controller.model.clear()
    inserted.clear()
    removed.clear()
    changed.clear()
  }

  function test_cleanup_requires_consent_even_with_an_old_retention_value() {
    fakeService.stateReady = true
    fakeService.trashRetentionDays = 7
    compare(controller.pruneExpired(), "disabled")
    compare(requests.length, 0)
    fakeService.trashCleanupConsent = true
    controller.pruneExpired()
    compare(requests[0].name, "trash-prune")
    compare(requests[0].arguments, ["--days", "7"])
  }

  function test_identical_refresh_keeps_existing_rows() {
    var entries = [entry("a", "a.txt"), entry("b", "b.txt")]
    controller.applyList({ ok: true, entries: entries })
    compare(controller.model.count, 2)
    verify(inserted.count > 0)

    inserted.clear()
    removed.clear()
    changed.clear()
    controller.applyList({ ok: true, entries: entries })

    compare(inserted.count, 0)
    compare(removed.count, 0)
    compare(changed.count, 0)
    compare(controller.model.get(0).entryId, "a")
    compare(controller.model.get(1).entryId, "b")
  }

  function test_file_operations_refresh_trash_even_before_a_store_is_watched() {
    history.operationCompleted({ operation: "copy", ok: true })
    compare(requests.length, 0)
    history.operationCompleted({ operation: "trash", ok: true })
    compare(requests.length, 1)
    compare(requests[0].name, "trash-list")
    fakeService.reply({ ok: true, entries: [entry("a", "a.txt")] })
    compare(controller.count, 1)
    history.operationCompleted({ operation: "undo", ok: true })
    fakeService.reply({ ok: true, entries: [] })
    compare(controller.count, 0)
  }

  function test_restore_notice_is_concise_and_expires() {
    controller.applyList({ ok: true, entries: [entry("a", "a.txt")] })
    controller.noticeDuration = 20
    controller.restore("a", "", false)
    compare(requests[0].name, "trash-restore")
    fakeService.reply({ ok: true, completed: 1 })

    compare(controller.notice, "Restored")
    tryCompare(controller, "notice", "", 200)
  }
}

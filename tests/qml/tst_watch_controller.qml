import QtQuick
import QtTest
import "../../controllers"

TestCase {
  name: "WatchControllerRegression"

  property var requests: []
  property var callbacks: []
  property int searchRestartCount: 0

  ListModel { id: rows }

  Item {
    id: fakeService
    property bool stateReady: false
    property bool open: false
    property string rootPath: "/tmp"
    property bool showHidden: false
    property string searchQuery: ""
    property int directoryBatchLimit: 8
    property var treeModel: rows
    property var gitRepoDirectories: ({})
    property bool trashMode: false
    property bool recentMode: false
    property bool drivesMode: false
    property var backendLimits: ({ watch_paths: 512 })
    property int treeRefreshCount: 0

    function parentDirectory(path) { return "/tmp" }
    function indexOfTreePath(path) { return -1 }
    function scheduleGitMetadataRefresh(reason, repoRoot) {}
    function refreshTree() { treeRefreshCount++ }
    function restartSearch() { searchRestartCount++ }
    function flushFilesystemEvents() { controller.flushFilesystemEvents() }
    function backendRequest(name, arguments, generation, callback) {
      requests.push({ name: name, arguments: arguments, generation: generation })
      callbacks.push(callback)
      return name + "-request"
    }
  }

  WatchController {
    id: controller
    service: fakeService
  }

  function init() {
    requests = []
    callbacks = []
    searchRestartCount = 0
    fakeService.searchQuery = ""
    fakeService.treeRefreshCount = 0
    controller.pendingWatchDirectories = []
    controller.watchRefreshQueue = []
    controller.activeWatchRefreshPaths = []
    controller.indexDirty = false
  }

  function completeInvalidation() {
    compare(callbacks.length, 1)
    callbacks.shift()({ ok: true })
  }

  function test_dirty_index_is_invalidated_without_an_active_search() {
    controller.indexDirty = true
    controller.flushFilesystemEvents()

    compare(requests.length, 1)
    compare(requests[0].name, "index-invalidate")
    verify(!controller.indexDirty)
    completeInvalidation()
    compare(searchRestartCount, 0)
  }

  function test_active_search_restarts_after_invalidation() {
    fakeService.searchQuery = "needle"
    controller.indexDirty = true
    controller.flushFilesystemEvents()

    completeInvalidation()
    compare(searchRestartCount, 1)
  }

  function test_cleared_search_does_not_restart_after_invalidation() {
    fakeService.searchQuery = "needle"
    controller.indexDirty = true
    controller.flushFilesystemEvents()
    fakeService.searchQuery = ""

    completeInvalidation()
    compare(searchRestartCount, 0)
  }

  function test_overflow_schedules_index_invalidation() {
    controller.receiveFilesystemEvent({ overflow: true })

    compare(fakeService.treeRefreshCount, 1)
    verify(controller.indexDirty)
    tryCompare(requests, "length", 1, 500)
    compare(requests[0].name, "index-invalidate")
    verify(!controller.indexDirty)
  }
}

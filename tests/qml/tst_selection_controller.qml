import QtQuick
import QtTest
import "../../controllers"

TestCase {
  name: "SelectionControllerRegression"

  QtObject {
    id: metadata
    function lookupPending(path) { return false }
    function reset() { fakeService.selectedMetadata = null }
  }

  QtObject {
    id: fakeService
    property string rootPath: "/fixture"
    property var treeModel: null
    property var searchModel: null
    property bool pickerActive: false
    property string pickerMode: ""
    property bool pickerSaveValidationBusy: false
    property bool pickerMultiple: true
    property int selectedFolderCount: 0
    property string selectionFolderColor: ""
    property bool open: true
    property string searchQuery: ""
    property bool quickNavActive: false
    property bool searchBusy: false
    property string searchBackend: ""
    property string searchFilterSummary: ""
    property int searchGitRepositoryCount: 0
    property string searchError: ""
    property var selectedMetadata: null
    property string selectedMetadataFingerprint: ""
    property var metadataControllerApi: metadata
    property bool metadataBusy: false
    property string metadataError: ""
    property int statRequests: 0
    property string lastStatPath: ""
    property string trashResource: "trash:///"
    property string recentResource: "recent:///"
    property string drivesResource: "drives:///"

    function requestStat(path) { statRequests++; lastStatPath = path }
    function remappedOperationPath(path, mappings) {
      for (var i = 0; i < mappings.length; i++) {
        var mapping = mappings[i]
        if (path === mapping.source || path.indexOf(mapping.source + "/") === 0)
          return mapping.destination + path.slice(mapping.source.length)
      }
      return path
    }
  }

  SelectionController { id: controller; service: fakeService }

  function entry(path) { return { path: path, name: path.split("/").pop(), statFingerprint: "old-stat" } }
  function init() {
    controller.clearSelection()
    fakeService.statRequests = 0
    fakeService.lastStatPath = ""
  }

  function test_rename_keeps_primary_and_anchor_on_the_new_path() {
    var old = entry("/fixture/before.txt")
    controller.applySelection([old], old, old.path)
    controller.remapPaths([{ source: old.path, destination: "/fixture/after.txt" }])
    compare(controller.selectedPath, "/fixture/after.txt")
    compare(controller.anchorPath, "/fixture/after.txt")
    compare(controller.selectedEntries[0].name, "after.txt")
    compare(controller.selectedEntries[0].statFingerprint, "")
    compare(fakeService.lastStatPath, "/fixture/after.txt")
    verify(!controller.isSelected(old.path))
    verify(controller.isSelected("/fixture/after.txt"))
  }

  function test_completion_does_not_steal_a_newer_selection() {
    var newer = entry("/fixture/newer.txt")
    controller.applySelection([newer], newer, newer.path)
    var requests = fakeService.statRequests
    controller.remapPaths([{ source: "/fixture/before.txt", destination: "/fixture/after.txt" }])
    compare(controller.selectedPath, newer.path)
    compare(fakeService.statRequests, requests)
  }

  function test_directory_rename_keeps_selected_children_and_unrelated_items() {
    var child = entry("/fixture/before/child.txt")
    var unrelated = entry("/fixture/other.txt")
    controller.applySelection([child, unrelated], child, unrelated.path)
    controller.remapPaths([{ source: "/fixture/before", destination: "/fixture/after" }])
    compare(controller.selectedCount, 2)
    compare(controller.selectedPath, "/fixture/after/child.txt")
    compare(controller.anchorPath, unrelated.path)
    verify(controller.isSelected(unrelated.path))
  }
}

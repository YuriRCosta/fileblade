import QtQuick
import QtTest
import "../../controllers" as Controllers

TestCase {
  id: testCase
  name: "ProjectControllerContext"

  property var subject: null

  QtObject {
    id: fakeService
    property string pluginDir: ""
    property string rootPath: "/repo"
    property string selectedPath: ""
    property var selectedMetadata: null
    property bool projectContext: false
  }

  Component {
    id: controllerComponent
    Controllers.ProjectController { service: fakeService }
  }

  function init() {
    fakeService.rootPath = "/repo"
    fakeService.selectedPath = ""
    fakeService.selectedMetadata = null
    fakeService.projectContext = false
    subject = controllerComponent.createObject(testCase)
    verify(subject !== null)
    subject.projectRoot = "/repo"
    subject.projectMarker = ".git"
    subject.resolvedAnchor = subject.anchor
  }

  function cleanup() {
    subject.destroy()
    subject = null
  }

  function test_selected_folder_is_the_default_context() {
    fakeService.selectedPath = "/repo/crates"
    fakeService.selectedMetadata = ({ path: "/repo/crates", is_dir: true })
    compare(subject.contextPath, "/repo/crates")
  }

  function test_file_selection_uses_the_open_folder() {
    fakeService.selectedPath = "/repo/README.md"
    fakeService.selectedMetadata = ({ path: "/repo/README.md", is_dir: false })
    compare(subject.contextPath, "/repo")
  }

  function test_git_project_mode_uses_the_resolved_repository() {
    fakeService.selectedPath = "/repo/crates"
    fakeService.selectedMetadata = ({ path: "/repo/crates", is_dir: true })
    subject.resolvedAnchor = subject.anchor
    fakeService.projectContext = true
    compare(subject.contextPath, "/repo")
  }

  function test_git_project_mode_falls_back_when_no_git_root_resolved() {
    fakeService.selectedPath = "/repo/crates"
    fakeService.selectedMetadata = ({ path: "/repo/crates", is_dir: true })
    subject.projectMarker = ".claude"
    subject.resolvedAnchor = subject.anchor
    fakeService.projectContext = true
    compare(subject.contextPath, "/repo/crates")
  }
}

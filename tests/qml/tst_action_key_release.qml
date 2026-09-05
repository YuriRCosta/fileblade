import QtQuick
import QtTest
import "../../ui" as Ui

TestCase {
  id: test
  name: "ActionKeyReleaseRouting"
  when: windowShown
  visible: true
  width: 100
  height: 100

  Ui.ActionKeyGuard { id: owner; releaseRoot: test.Window.window ? test.Window.window.contentItem : null }
  Item {
    id: opener
    width: 80
    height: 80
    Keys.onPressed: function(event) {
      if (event.key !== Qt.Key_Return) return
      verify(!owner.isRepeat(event))
      opener.focus = false
      opener.visible = false
      test.Window.window.contentItem.forceActiveFocus()
      event.accepted = true
    }
  }

  function test_release_at_window_root_clears_a_key_from_a_closed_dialog() {
    owner.reset()
    opener.forceActiveFocus()
    verify(opener.activeFocus)
    keyPress(Qt.Key_Return)
    verify(!opener.activeFocus)
    verify(owner.held[String(Qt.Key_Return)])
    keyRelease(Qt.Key_Return)
    tryVerify(function() { return !owner.held[String(Qt.Key_Return)] })
    verify(!owner.isRepeat({key: Qt.Key_Return, isAutoRepeat: false}))
  }
}

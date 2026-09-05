import QtQuick
import QtTest
import "../../ui" as Ui

TestCase {
  name: "ActionKeyGuard"
  Ui.ActionKeyGuard { id: guard }
  Ui.ActionKeyGuard { id: owner }
  Ui.ActionKeyGuard { id: dialogKeys; shared: owner }

  function event(key, repeat) { return { key: key, isAutoRepeat: !!repeat } }
  function init() { guard.shared = null; guard.active = true; guard.reset(); owner.active = true; owner.reset() }

  function test_unmarked_repeats_wait_for_release() {
    verify(!guard.isRepeat(event(Qt.Key_Return)))
    verify(guard.isRepeat(event(Qt.Key_Return)))
    verify(guard.isRepeat(event(Qt.Key_Return, true)))
    guard.release(event(Qt.Key_Return, true))
    verify(guard.isRepeat(event(Qt.Key_Return)))
    guard.release(event(Qt.Key_Return))
    wait(30)
    verify(!guard.isRepeat(event(Qt.Key_Return)))
  }

  function test_keys_are_independent() {
    verify(!guard.isRepeat(event(Qt.Key_Return)))
    verify(!guard.isRepeat(event(Qt.Key_Escape)))
    guard.release(event(Qt.Key_Escape))
    wait(30)
    verify(guard.isRepeat(event(Qt.Key_Return)))
    verify(!guard.isRepeat(event(Qt.Key_Escape)))
  }

  function test_synthetic_release_press_pair_is_still_a_repeat() {
    verify(!guard.isRepeat(event(Qt.Key_Return)))
    guard.release(event(Qt.Key_Return))
    verify(guard.isRepeat(event(Qt.Key_Return)))
    wait(30)
    verify(guard.isRepeat(event(Qt.Key_Return)))
    guard.release(event(Qt.Key_Return))
    wait(30)
    verify(!guard.isRepeat(event(Qt.Key_Return)))
  }

  function test_focus_loss_discards_keys_released_elsewhere() {
    verify(!guard.isRepeat(event(Qt.Key_Delete)))
    guard.active = false
    guard.active = true
    verify(!guard.isRepeat(event(Qt.Key_Delete)))
  }

  function test_shared_owner_survives_popup_to_pane_focus_transfer() {
    guard.shared = owner
    verify(!guard.isRepeat(event(Qt.Key_Return)))
    guard.active = false
    verify(owner.isRepeat(event(Qt.Key_Return)))
    guard.release(event(Qt.Key_Return))
    wait(30)
    verify(!owner.isRepeat(event(Qt.Key_Return)))
    owner.active = false
    verify(!guard.isRepeat(event(Qt.Key_Return)))
  }

  function test_confirmation_releases_the_opening_key_for_the_tree() {
    guard.shared = owner
    verify(!guard.isRepeat(event(Qt.Key_D)))
    guard.active = false
    verify(dialogKeys.isRepeat(event(Qt.Key_D)))
    dialogKeys.release(event(Qt.Key_D))
    wait(30)
    guard.active = true
    verify(!guard.isRepeat(event(Qt.Key_D)))
    verify(!dialogKeys.isRepeat(event(Qt.Key_Return)))
    dialogKeys.active = false
    verify(guard.isRepeat(event(Qt.Key_Return)))
    guard.release(event(Qt.Key_Return))
    wait(30)
    verify(!dialogKeys.isRepeat(event(Qt.Key_Return)))
    dialogKeys.active = true
  }
}

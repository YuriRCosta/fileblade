import QtQuick
import QtTest
import "../../modules/notes/KeyPlan.js" as KeyPlan

TestCase {
  name: "NotesKeyPlanRegression"

  function act(key, modifiers) {
    return KeyPlan.editorAction(key, modifiers === undefined ? Qt.NoModifier : modifiers)
  }

  function test_tab_cycle_chords_are_never_consumed() {
    compare(KeyPlan.isTabCycle(Qt.Key_Tab, Qt.ControlModifier), true)
    compare(KeyPlan.isTabCycle(Qt.Key_Backtab, Qt.ControlModifier), true)
    compare(KeyPlan.isTabCycle(Qt.Key_Backtab, Qt.ControlModifier | Qt.ShiftModifier), true)
    compare(act(Qt.Key_Tab, Qt.ControlModifier), "")
    compare(act(Qt.Key_Backtab, Qt.ControlModifier), "")
    compare(act(Qt.Key_Backtab, Qt.ControlModifier | Qt.ShiftModifier), "")
  }

  function test_slot_movement() {
    compare(act(Qt.Key_Tab), "focus-next")
    compare(act(Qt.Key_Backtab), "focus-previous")
    compare(act(Qt.Key_Backtab, Qt.ShiftModifier), "focus-previous")
  }

  function test_escape_flushes_and_returns() {
    compare(act(Qt.Key_Escape), "flush-and-close")
  }

  function test_editing_chords_stay_with_the_editor() {
    var chords = [Qt.Key_C, Qt.Key_V, Qt.Key_X, Qt.Key_A, Qt.Key_Z, Qt.Key_Y, Qt.Key_Left, Qt.Key_Right,
                  Qt.Key_Home, Qt.Key_End, Qt.Key_Up, Qt.Key_Down, Qt.Key_D, Qt.Key_U]
    for (var index = 0; index < chords.length; index++) {
      compare(act(chords[index], Qt.ControlModifier), "")
      compare(act(chords[index], Qt.NoModifier), "")
    }
  }

  function test_alt_and_meta_are_ignored() {
    compare(act(Qt.Key_Tab, Qt.AltModifier), "")
    compare(act(Qt.Key_Escape, Qt.MetaModifier), "")
  }

  function test_plain_typing_is_never_intercepted() {
    var typed = [Qt.Key_J, Qt.Key_K, Qt.Key_H, Qt.Key_L, Qt.Key_G, Qt.Key_R, Qt.Key_O, Qt.Key_Slash, Qt.Key_Space]
    for (var index = 0; index < typed.length; index++) {
      compare(act(typed[index]), "")
      compare(act(typed[index], Qt.ShiftModifier), "")
    }
  }

}

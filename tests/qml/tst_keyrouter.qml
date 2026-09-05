import QtQuick
import QtTest
import "../../lib/KeyRouter.js" as KeyRouter

TestCase {
  function test_escape_is_one_shot_without_disabling_tab_or_row_repeat() {
    verify(KeyRouter.ignoresAutoRepeat("focus-previous", Qt.Key_Escape))
    verify(!KeyRouter.ignoresAutoRepeat("focus-previous", Qt.Key_Backtab))
    verify(!KeyRouter.ignoresAutoRepeat("focus-next", Qt.Key_Tab))
    verify(!KeyRouter.ignoresAutoRepeat("next", Qt.Key_Down))
    verify(!KeyRouter.ignoresAutoRepeat("", Qt.Key_Escape))
  }
  name: "KeyRouter"

  function press(key, modifiers) {
    return { key: key, modifiers: modifiers === undefined ? Qt.NoModifier : modifiers }
  }

  readonly property var one: ({ count: 1, deleted: false, directory: false, visual: false })
  readonly property var none: ({ count: 0, deleted: false, directory: false, visual: false })

  function test_list_actions_data() {
    return [
      { tag: "j moves down", key: Qt.Key_J, modifiers: Qt.NoModifier, tree: true, state: one, expected: "next" },
      { tag: "k moves up", key: Qt.Key_K, modifiers: Qt.NoModifier, tree: true, state: one, expected: "previous" },
      { tag: "shift+g goes last", key: Qt.Key_G, modifiers: Qt.ShiftModifier, tree: true, state: one, expected: "last" },
      { tag: "h goes to the parent", key: Qt.Key_H, modifiers: Qt.NoModifier, tree: true, state: one, expected: "up" },
      { tag: "h also goes up outside the tree", key: Qt.Key_H, modifiers: Qt.NoModifier, tree: false, state: one, expected: "up" },
      { tag: "l enters the folder", key: Qt.Key_L, modifiers: Qt.NoModifier, tree: true, state: one, expected: "open" },
      { tag: "ctrl+b pages up", key: Qt.Key_B, modifiers: Qt.ControlModifier, tree: true, state: one, expected: "page-previous" },
      { tag: "ctrl+shift+b toggles search layout", key: Qt.Key_B, modifiers: Qt.ControlModifier | Qt.ShiftModifier, tree: false, state: one, expected: "layout" },
      { tag: "ctrl+l is the location field", key: Qt.Key_L, modifiers: Qt.ControlModifier, tree: true, state: one, expected: "location" },
      { tag: "period toggles hidden like yazi", key: Qt.Key_Period, modifiers: Qt.NoModifier, tree: true, state: one, expected: "hidden" },
      { tag: "shift+h toggles hidden like lazyvim", key: Qt.Key_H, modifiers: Qt.ShiftModifier, tree: true, state: one, expected: "hidden" },
      { tag: "ctrl+h toggles hidden", key: Qt.Key_H, modifiers: Qt.ControlModifier, tree: true, state: one, expected: "hidden" },
      { tag: "backspace goes back", key: Qt.Key_Backspace, modifiers: Qt.NoModifier, tree: true, state: one, expected: "back" },
      { tag: "alt+up goes to the parent", key: Qt.Key_Up, modifiers: Qt.AltModifier, tree: true, state: one, expected: "up" },
      { tag: "u undoes", key: Qt.Key_U, modifiers: Qt.NoModifier, tree: true, state: one, expected: "undo" },
      { tag: "ctrl+shift+z redoes", key: Qt.Key_Z, modifiers: Qt.ControlModifier | Qt.ShiftModifier, tree: true, state: one, expected: "redo" },
      { tag: "bare z is reserved for folds", key: Qt.Key_Z, modifiers: Qt.NoModifier, tree: true, state: one, expected: "" },
      { tag: "shift z is quick nav", key: Qt.Key_Z, modifiers: Qt.ShiftModifier, tree: true, state: one, expected: "quicknav" },
      { tag: "slash searches", key: Qt.Key_Slash, modifiers: Qt.NoModifier, tree: true, state: one, expected: "search" },
      { tag: "ctrl+f is deep search", key: Qt.Key_F, modifiers: Qt.ControlModifier, tree: true, state: one, expected: "deep" },
      { tag: "v enters visual", key: Qt.Key_V, modifiers: Qt.NoModifier, tree: true, state: one, expected: "visual" },
      { tag: "tab moves slot focus", key: Qt.Key_Tab, modifiers: Qt.NoModifier, tree: true, state: one, expected: "focus-next" },
      { tag: "enter activates", key: Qt.Key_Return, modifiers: Qt.NoModifier, tree: true, state: one, expected: "activate" },
      { tag: "o opens rather than toggles", key: Qt.Key_O, modifiers: Qt.NoModifier, tree: true, state: one, expected: "open" },
      { tag: "o opens outside the tree", key: Qt.Key_O, modifiers: Qt.NoModifier, tree: false, state: one, expected: "open" },
      { tag: "shift+right expands the subtree", key: Qt.Key_Right, modifiers: Qt.ShiftModifier, tree: true, state: one, expected: "expand-recursive" },
      { tag: "shift+left collapses the subtree", key: Qt.Key_Left, modifiers: Qt.ShiftModifier, tree: true, state: one, expected: "collapse-recursive" },
      { tag: "shift+right does nothing outside the tree", key: Qt.Key_Right, modifiers: Qt.ShiftModifier, tree: false, state: one, expected: "" },
      { tag: "ctrl+right is not recursive expansion", key: Qt.Key_Right, modifiers: Qt.ControlModifier, tree: true, state: one, expected: "" },
      { tag: "alt+left stays browser navigation", key: Qt.Key_Left, modifiers: Qt.AltModifier, tree: true, state: one, expected: "back" },
      { tag: "shift+enter opens with", key: Qt.Key_Return, modifiers: Qt.ShiftModifier, tree: true, state: one, expected: "open-with" },
      { tag: "e opens the editor for one item", key: Qt.Key_E, modifiers: Qt.NoModifier, tree: true, state: one, expected: "editor" },
      { tag: "ctrl+c copies", key: Qt.Key_C, modifiers: Qt.ControlModifier, tree: true, state: one, expected: "copy" },
      { tag: "ctrl+x cuts", key: Qt.Key_X, modifiers: Qt.ControlModifier, tree: true, state: one, expected: "cut" },
      { tag: "ctrl+v pastes", key: Qt.Key_V, modifiers: Qt.ControlModifier, tree: true, state: one, expected: "paste" },
      { tag: "delete trashes a selection", key: Qt.Key_Delete, modifiers: Qt.NoModifier, tree: true, state: one, expected: "trash" },
      { tag: "delete with nothing selected is nothing", key: Qt.Key_Delete, modifiers: Qt.NoModifier, tree: true, state: none, expected: "" },
      { tag: "d trashes like nvim", key: Qt.Key_D, modifiers: Qt.NoModifier, tree: true, state: one, expected: "trash" },
      { tag: "r renames one item", key: Qt.Key_R, modifiers: Qt.NoModifier, tree: true, state: one, expected: "rename" },
      { tag: "a is new file", key: Qt.Key_A, modifiers: Qt.NoModifier, tree: true, state: one, expected: "new-file" },
      { tag: "ctrl+shift+n is new folder", key: Qt.Key_N, modifiers: Qt.ControlModifier | Qt.ShiftModifier, tree: true, state: one, expected: "new-folder" },
      { tag: "m opens actions", key: Qt.Key_M, modifiers: Qt.NoModifier, tree: true, state: one, expected: "actions" },
      { tag: "escape dismisses", key: Qt.Key_Escape, modifiers: Qt.NoModifier, tree: true, state: one, expected: "dismiss" },
      { tag: "escape leaves visual", key: Qt.Key_Escape, modifiers: Qt.NoModifier, tree: true, state: { count: 1, deleted: false, directory: false, visual: true }, expected: "visual-exit" },
      { tag: "j extends in visual", key: Qt.Key_J, modifiers: Qt.NoModifier, tree: true, state: { count: 1, deleted: false, directory: false, visual: true }, expected: "next-extend" },
      { tag: "q closes", key: Qt.Key_Q, modifiers: Qt.NoModifier, tree: true, state: one, expected: "close" }
    ]
  }

  function test_list_actions(data) {
    compare(KeyRouter.listAction(press(data.key, data.modifiers), data.tree, data.state), data.expected)
  }

  function test_property_actions_data() {
    return [
      { tag: "escape closes the blade", key: Qt.Key_Escape, modifiers: Qt.NoModifier, state: { actionable: true, count: 1, deleted: false, directory: false }, expected: "dismiss" },
      { tag: "m opens actions", key: Qt.Key_M, modifiers: Qt.NoModifier, state: { actionable: true, count: 1, deleted: false, directory: false }, expected: "actions" },
      { tag: "period toggles hidden", key: Qt.Key_Period, modifiers: Qt.NoModifier, state: { actionable: true, count: 1, deleted: false, directory: false }, expected: "hidden" },
      { tag: "shift+h toggles hidden", key: Qt.Key_H, modifiers: Qt.ShiftModifier, state: { actionable: true, count: 1, deleted: false, directory: false }, expected: "hidden" },
      { tag: "j scrolls down", key: Qt.Key_J, modifiers: Qt.NoModifier, state: { actionable: true, count: 1, deleted: false, directory: false }, expected: "scroll-down" },
      { tag: "r reveals", key: Qt.Key_R, modifiers: Qt.NoModifier, state: { actionable: true, count: 1, deleted: false, directory: false }, expected: "reveal" },
      { tag: "ctrl+d pages down", key: Qt.Key_D, modifiers: Qt.ControlModifier, state: { actionable: true, count: 1, deleted: false, directory: false }, expected: "page-down" },
      { tag: "ctrl+b pages up", key: Qt.Key_B, modifiers: Qt.ControlModifier, state: { actionable: true, count: 1, deleted: false, directory: false }, expected: "page-up" },
      { tag: "enter activates when actionable", key: Qt.Key_Return, modifiers: Qt.NoModifier, state: { actionable: true, count: 1, deleted: false, directory: false }, expected: "activate" },
      { tag: "o opens in Properties", key: Qt.Key_O, modifiers: Qt.NoModifier, state: { actionable: true, count: 1, deleted: false, directory: false }, expected: "open" },
      { tag: "enter does nothing without an entry", key: Qt.Key_Return, modifiers: Qt.NoModifier, state: { actionable: false, count: 0, deleted: false, directory: false }, expected: "" }
    ]
  }

  function test_property_actions(data) {
    compare(KeyRouter.propertyAction(press(data.key, data.modifiers), data.state), data.expected)
  }

  function test_auto_repeat_is_ignored_for_destructive_actions() {
    verify(KeyRouter.ignoresAutoRepeat("trash"))
    verify(KeyRouter.ignoresAutoRepeat("paste"))
    verify(KeyRouter.ignoresAutoRepeat("dismiss"))
    verify(KeyRouter.ignoresAutoRepeat("visual-exit"))
    verify(KeyRouter.ignoresAutoRepeat("open"))
    verify(KeyRouter.ignoresAutoRepeat("expand-recursive"))
    verify(KeyRouter.ignoresAutoRepeat("collapse-recursive"))
    verify(!KeyRouter.ignoresAutoRepeat("next"))
  }

  function test_artifact_navigation_reuses_file_tree_keys() {
    for (var key of [Qt.Key_J, Qt.Key_K, Qt.Key_Home, Qt.Key_End, Qt.Key_Left, Qt.Key_Right, Qt.Key_PageDown, Qt.Key_Tab])
      compare(KeyRouter.artifactAction(press(key)), KeyRouter.movementAction(press(key), true, false))
    compare(KeyRouter.artifactAction(press(Qt.Key_D, Qt.ControlModifier)), "page-next")
    compare(KeyRouter.artifactAction(press(Qt.Key_Escape)), "dismiss")
    compare(KeyRouter.artifactAction(press(Qt.Key_Backtab, Qt.ShiftModifier)), "focus-previous")
    compare(KeyRouter.artifactAction(press(Qt.Key_U, Qt.ControlModifier)), "page-previous")
    compare(KeyRouter.artifactAction(press(Qt.Key_B, Qt.ControlModifier)), "page-previous")
    compare(KeyRouter.artifactAction(press(Qt.Key_G, Qt.ShiftModifier)), "last")
    compare(KeyRouter.artifactAction(press(Qt.Key_Right, Qt.ShiftModifier)), "expand-recursive")
    compare(KeyRouter.artifactAction(press(Qt.Key_Left, Qt.ShiftModifier)), "collapse-recursive")
    compare(KeyRouter.artifactAction(press(Qt.Key_O)), "open")
  }

  function test_artifact_tree_leaves_shifted_domain_shortcuts_to_the_module() {
    for (var key of [Qt.Key_D, Qt.Key_F, Qt.Key_R, Qt.Key_Y])
      compare(KeyRouter.artifactAction(press(key, Qt.ShiftModifier)), "")
    compare(KeyRouter.artifactAction(press(Qt.Key_D)), "action")
    compare(KeyRouter.artifactAction(press(Qt.Key_F)), "filter")
    compare(KeyRouter.artifactAction(press(Qt.Key_Return, Qt.ShiftModifier)), "open-with")
    compare(KeyRouter.artifactAction(press(Qt.Key_Tab, Qt.ControlModifier)), "")
  }
}

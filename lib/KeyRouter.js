.pragma library

function has(modifiers, flag) {
  return !!(modifiers & flag)
}

function exactControl(modifiers) {
  return has(modifiers, Qt.ControlModifier)
    && !has(modifiers, Qt.AltModifier | Qt.MetaModifier | Qt.ShiftModifier)
}

function noModifiers(modifiers) {
  return !has(modifiers, Qt.ControlModifier | Qt.AltModifier | Qt.MetaModifier | Qt.ShiftModifier)
}

function commandModifier(modifiers) {
  return has(modifiers, Qt.ControlModifier | Qt.MetaModifier) && !has(modifiers, Qt.AltModifier)
}

function ignoresAutoRepeat(action, key) {
  return (key === Qt.Key_Escape && action !== "") || ["undo", "redo", "skip-refused", "trash", "paste", "cut", "copy", "rename", "new-file", "new-folder",
    "activate", "open", "help", "up", "expand", "collapse", "expand-recursive", "collapse-recursive", "expand-all", "collapse-all", "action", "open-with", "editor", "edit", "reveal", "dismiss", "visual-exit", "tree"].indexOf(action) >= 0
}

function historyAction(event) {
  if (event.key === Qt.Key_U && noModifiers(event.modifiers)) return "undo"
  if (event.key === Qt.Key_U && has(event.modifiers, Qt.ShiftModifier)
      && !has(event.modifiers, Qt.ControlModifier | Qt.AltModifier | Qt.MetaModifier)) return "skip-refused"
  if (!commandModifier(event.modifiers)) return ""
  if (event.key === Qt.Key_Z) return has(event.modifiers, Qt.ShiftModifier) ? "redo" : "undo"
  if (event.key === Qt.Key_Y) return "redo"
  if (event.key === Qt.Key_R && has(event.modifiers, Qt.ControlModifier)) return "redo"
  return ""
}

function browserAction(event) {
  if (event.key === Qt.Key_Period && noModifiers(event.modifiers)) return "hidden"
  if (event.key === Qt.Key_H && has(event.modifiers, Qt.ShiftModifier)
      && !has(event.modifiers, Qt.ControlModifier | Qt.AltModifier | Qt.MetaModifier)) return "hidden"
  var control = {}
  control[Qt.Key_L] = "location"
  control[Qt.Key_H] = "hidden"
  if (exactControl(event.modifiers) && control[event.key]) return control[event.key]
  if (event.key === Qt.Key_Backspace && noModifiers(event.modifiers)) return "back"
  if (!has(event.modifiers, Qt.AltModifier)) return ""
  var navigation = {}
  navigation[Qt.Key_Left] = "back"
  navigation[Qt.Key_Right] = "forward"
  navigation[Qt.Key_Up] = "up"
  navigation[Qt.Key_Home] = "home"
  return navigation[event.key] || ""
}

function pageMovement(event) {
  if (exactControl(event.modifiers)) {
    if (event.key === Qt.Key_D) return "page-next"
    if (event.key === Qt.Key_U || event.key === Qt.Key_B) return "page-previous"
  }
  if (has(event.modifiers, Qt.ControlModifier | Qt.AltModifier | Qt.MetaModifier)) return ""
  if (event.key === Qt.Key_PageDown) return "page-next"
  if (event.key === Qt.Key_PageUp) return "page-previous"
  return ""
}

function slotFocusMovement(event) {
  if (event.key === Qt.Key_Tab && noModifiers(event.modifiers)) return "focus-next"
  if (event.key === Qt.Key_Backtab
      && !has(event.modifiers, Qt.ControlModifier | Qt.AltModifier | Qt.MetaModifier)) return "focus-previous"
  return ""
}

function rowMovement(event) {
  var movement = {}
  movement[Qt.Key_Down] = "next"
  movement[Qt.Key_J] = "next"
  movement[Qt.Key_Up] = "previous"
  movement[Qt.Key_K] = "previous"
  movement[Qt.Key_Home] = "first"
  movement[Qt.Key_End] = "last"
  if (event.key === Qt.Key_G) return has(event.modifiers, Qt.ShiftModifier) ? "last" : "first"
  return movement[event.key] || ""
}

function hierarchyMovement(event, treeMode) {
  if (event.modifiers === Qt.ShiftModifier) {
    if (treeMode && event.key === Qt.Key_Right) return "expand-recursive"
    if (treeMode && event.key === Qt.Key_Left) return "collapse-recursive"
    return ""
  }
  if (!noModifiers(event.modifiers)) return ""
  if (event.key === Qt.Key_Left || event.key === Qt.Key_H) return "up"
  if (event.key === Qt.Key_Right || event.key === Qt.Key_L) return "open"
  return ""
}

function movementAction(event, treeMode, visual) {
  var action = pageMovement(event)
  if (action) return visual ? action + "-extend" : action
  action = slotFocusMovement(event)
  if (action) return action
  action = rowMovement(event)
  if (action) return visual ? action + "-extend" : action
  return hierarchyMovement(event, treeMode)
}

function nvimAction(event, state) {
  if (event.key === Qt.Key_R && has(event.modifiers, Qt.ShiftModifier)) return "refresh"
  if (!noModifiers(event.modifiers)) return ""
  var single = state.count === 1 && !state.deleted
  var some = state.count > 0 && !state.deleted
  var keys = {}
  keys[Qt.Key_M] = some ? "actions" : ""
  keys[Qt.Key_A] = "new-file"
  keys[Qt.Key_R] = single ? "rename" : ""
  keys[Qt.Key_D] = some ? "trash" : ""
  keys[Qt.Key_Y] = some ? "copy" : ""
  keys[Qt.Key_C] = some ? "copy" : ""
  keys[Qt.Key_X] = some ? "cut" : ""
  keys[Qt.Key_P] = "paste"
  return keys[event.key] || ""
}

function selectionAction(event, state) {
  if (event.key === Qt.Key_Space && has(event.modifiers, Qt.ControlModifier)) return "toggle-selection"
  if (event.key === Qt.Key_A && has(event.modifiers, Qt.ControlModifier)) return "select-all"
  return clipboardAction(event, state)
}

function clipboardAction(event, state) {
  if (!commandModifier(event.modifiers)) return ""
  if (event.key === Qt.Key_C && !state.deleted) return "copy"
  if (event.key === Qt.Key_X && !state.deleted) return "cut"
  if (event.key === Qt.Key_V) return "paste"
  return ""
}

function menuAction(event, state) {
  if (event.key === Qt.Key_N && has(event.modifiers, Qt.ControlModifier)
      && !has(event.modifiers, Qt.AltModifier | Qt.MetaModifier))
    return has(event.modifiers, Qt.ShiftModifier) ? "new-folder" : "new-file"
  if (event.key === Qt.Key_F2 && state.count === 1 && !state.deleted) return "rename"
  if (event.key === Qt.Key_Delete && state.count > 0 && !state.deleted) return "trash"
  if (event.key === Qt.Key_Menu && state.count > 0 && !state.deleted) return "actions"
  return ""
}

function propertyMenuAction(event, state) {
  if (event.key === Qt.Key_N && has(event.modifiers, Qt.ControlModifier)
      && !has(event.modifiers, Qt.AltModifier | Qt.MetaModifier))
    return has(event.modifiers, Qt.ShiftModifier) ? "new-folder" : "new-file"
  if (event.key === Qt.Key_F2 && state.actionable) return "rename"
  if (event.key === Qt.Key_Delete && state.count > 0 && !state.deleted) return "trash"
  if ((event.key === Qt.Key_Menu || (event.key === Qt.Key_M && noModifiers(event.modifiers))) && state.count > 0 && !state.deleted) return "actions"
  return ""
}

function activationAction(event, state) {
  var enter = event.key === Qt.Key_Return || event.key === Qt.Key_Enter
  if (enter && has(event.modifiers, Qt.ShiftModifier) && state.count === 1 && !state.directory) return "open-with"
  if (enter) return "activate"
  if (has(event.modifiers, Qt.ControlModifier | Qt.AltModifier | Qt.MetaModifier)) return ""
  if (event.key === Qt.Key_O && noModifiers(event.modifiers)) return "open"
  if (event.key === Qt.Key_E && state.count === 1) return "editor"
  return ""
}

function artifactAction(event) {
  if (event.key === Qt.Key_Question && !has(event.modifiers, Qt.ControlModifier | Qt.AltModifier | Qt.MetaModifier)) return "help"
  var action = slotFocusMovement(event) || pageMovement(event)
  if (action) return action
  if (commandModifier(event.modifiers)) {
    if (event.key === Qt.Key_C) return "copy"
    if (event.key === Qt.Key_X) return "cut"
    return ""
  }
  if (has(event.modifiers, Qt.ControlModifier | Qt.AltModifier | Qt.MetaModifier)) return ""
  if (event.key === Qt.Key_G) return rowMovement(event)
  if (event.modifiers === Qt.ShiftModifier && (event.key === Qt.Key_Return || event.key === Qt.Key_Enter)) return "open-with"
  action = hierarchyMovement(event, true)
  if (action) return action
  if (!noModifiers(event.modifiers)) return ""
  action = rowMovement(event)
  if (action) return action
  var keys = {}
  keys[Qt.Key_Escape] = "dismiss"
  keys[Qt.Key_Slash] = "search"
  keys[Qt.Key_Return] = "activate"
  keys[Qt.Key_Enter] = "activate"
  keys[Qt.Key_O] = "open"
  keys[Qt.Key_R] = "reveal"
  keys[Qt.Key_D] = "action"
  keys[Qt.Key_Delete] = "action"
  keys[Qt.Key_F] = "filter"
  keys[Qt.Key_S] = "sort"
  keys[Qt.Key_Menu] = "menu"
  keys[Qt.Key_M] = "menu"
  keys[Qt.Key_Y] = "copy"
  keys[Qt.Key_X] = "cut"
  keys[Qt.Key_E] = "edit"
  keys[Qt.Key_F2] = "rename"
  return keys[event.key] || ""
}

function listAction(event, treeMode, state) {
  if (event.key === Qt.Key_Question && !has(event.modifiers, Qt.ControlModifier | Qt.AltModifier | Qt.MetaModifier)) return "help"
  var action = browserAction(event) || historyAction(event)
  if (action) return action
  action = listModeAction(event, treeMode)
  if (action) return action
  action = movementAction(event, treeMode, !!state.visual)
  if (action) return action
  action = activationAction(event, state) || selectionAction(event, state) || menuAction(event, state) || nvimAction(event, state)
  if (action) return action
  var simple = {}
  simple[Qt.Key_Escape] = state.visual ? "visual-exit" : "dismiss"
  simple[Qt.Key_Q] = "close"
  return simple[event.key] || ""
}

function listModeAction(event, treeMode) {
  if (event.key === Qt.Key_Z && event.modifiers === Qt.ShiftModifier) return "quicknav"
  if (event.key === Qt.Key_Slash && !has(event.modifiers, Qt.ControlModifier)) return "search"
  if (event.key === Qt.Key_B && event.modifiers === (Qt.ControlModifier | Qt.ShiftModifier)) return "layout"
  if (event.key === Qt.Key_P && exactControl(event.modifiers)) return "picker"
  if (event.key === Qt.Key_F && exactControl(event.modifiers)) return "deep"
  if (event.key === Qt.Key_V && !has(event.modifiers, Qt.ControlModifier | Qt.AltModifier | Qt.MetaModifier)) return "visual"
  return ""
}

function propertyMovement(event) {
  var page = pageMovement(event)
  if (page) return page === "page-next" ? "page-down" : "page-up"
  var focus = slotFocusMovement(event)
  if (focus) return focus
  if (has(event.modifiers, Qt.ControlModifier | Qt.AltModifier | Qt.MetaModifier)) return ""
  var movement = {}
  movement[Qt.Key_Up] = "scroll-up"
  movement[Qt.Key_K] = "scroll-up"
  movement[Qt.Key_Down] = "scroll-down"
  movement[Qt.Key_J] = "scroll-down"
  movement[Qt.Key_Home] = "first"
  movement[Qt.Key_End] = "last"
  movement[Qt.Key_Escape] = "dismiss"
  movement[Qt.Key_Left] = "tree"
  movement[Qt.Key_H] = "tree"
  movement[Qt.Key_Slash] = "search"
  if (event.key === Qt.Key_G) return has(event.modifiers, Qt.ShiftModifier) ? "last" : "first"
  return movement[event.key] || ""
}

function propertyAction(event, state) {
  if (event.key === Qt.Key_Question && !has(event.modifiers, Qt.ControlModifier | Qt.AltModifier | Qt.MetaModifier)) return "help"
  var action = browserAction(event) || historyAction(event) || propertyMovement(event)
  if (action) return action
  action = activationAction(event, { count: state.actionable ? 1 : 0, directory: state.directory })
  if (action && state.actionable) return action
  return clipboardAction(event, state) || propertyMenuAction(event, state) || (event.key === Qt.Key_R && state.actionable ? "reveal" : "") || (event.key === Qt.Key_Q ? "close" : "")
}

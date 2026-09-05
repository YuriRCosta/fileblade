.pragma library

function has(modifiers, flag) {
  return !!(modifiers & flag)
}

function isTabCycle(key, modifiers) {
  if (!has(modifiers, Qt.ControlModifier)) return false
  return key === Qt.Key_Tab || key === Qt.Key_Backtab
}

function editorAction(key, modifiers) {
  if (has(modifiers, Qt.ControlModifier) || has(modifiers, Qt.AltModifier) || has(modifiers, Qt.MetaModifier)) return ""
  if (key === Qt.Key_Tab) return "focus-next"
  if (key === Qt.Key_Backtab) return "focus-previous"
  if (key === Qt.Key_Escape) return "flush-and-close"
  return ""
}

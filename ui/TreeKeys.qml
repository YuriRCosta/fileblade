import QtQuick
import "../lib/KeyBindings.js" as KeyBindings

QtObject {
  property bool active: false
  property var plan: KeyBindings.compile({})
  property string scope: "files-tree"
  property var sequence: []
  readonly property bool pending: sequence.length > 0
  readonly property var entries: plan.entries.filter(function(entry) { return KeyBindings.actionFor(entry.action, scope) !== "" })
  readonly property string hint: {
    var matching = pending ? entries.filter(function(entry) { return KeyBindings.prefix(sequence, entry.keys) }) : []
    return matching.length ? matching[0].text.split(/\s+/).slice(0, sequence.length).join(" ") : ""
  }
  property int commandKey: 0

  onActiveChanged: reset()
  onEntriesChanged: reset()

  function reset() { sequence = []; commandKey = 0 }

  // The caller shares its action-key guard, so held continuations cannot
  // turn into ordinary open/copy/refresh actions after completing a fold.
  function action(event, repeated, fallback) {
    if (!active) return fallback || ""
    if ([Qt.Key_Shift, Qt.Key_Control, Qt.Key_Alt, Qt.Key_Meta].indexOf(event.key) >= 0)
      return pending ? "key-prefix" : ""
    if (event.key === commandKey && repeated) return "key-prefix"
    if (!pending && scope === "properties" && fallback === "up" && plan.custom.indexOf("up") < 0) return fallback
    commandKey = 0
    var wasPending = pending
    if (pending) {
      if (repeated) return "key-prefix"
      if (event.key === Qt.Key_Escape) { reset(); return "key-cancel" }
    }
    var next = sequence.concat([KeyBindings.signature(event.key, event.modifiers)])
    var matches = entries.filter(function(entry) { return KeyBindings.prefix(next, entry.keys) })
    if (!wasPending && matches.length === 0 && event.modifiers === Qt.ShiftModifier) {
      var unshifted = KeyBindings.signature(event.key, Qt.NoModifier)
      matches = entries.filter(function(entry) {
        return ["next", "previous", "first", "last"].indexOf(entry.action) >= 0 && entry.keys.length === 1 && entry.keys[0] === unshifted
      })
    }
    if (matches.length) {
      if (matches[0].keys.length === next.length) {
        sequence = []
        if (wasPending) commandKey = event.key
        return KeyBindings.actionFor(matches[0].action, scope)
      }
      if (!repeated) sequence = next
      return "key-prefix"
    }
    if (wasPending) { reset(); return "key-cancel" }
    return KeyBindings.legacyBlocked(fallback, scope) ? "key-unbound" : (fallback || "")
  }
}

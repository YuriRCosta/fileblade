.pragma library

var INTERACTIVE_COMMANDS = [
  "active-window",
  "children-batch",
  "children-window",
  "focus-direction",
  "focus-window",
  "hover-target",
  "hypr-option",
  "project-root",
  "stat",
  "stat-batch"
]

function maximumConcurrency(limits) {
  var value = Number(limits && limits.concurrency)
  if (!isFinite(value) || value < 1) value = 16
  return Math.max(1, Math.floor(value))
}

function backgroundCapacity(limits) {
  return Math.max(1, maximumConcurrency(limits) - 1)
}

function interactive(command, options) {
  var requested = String(options && options.priority || "")
  if (requested === "interactive") return true
  if (requested === "background") return false
  return INTERACTIVE_COMMANDS.indexOf(String(command || "")) >= 0
}

function canTransmit(isInteractive, inFlight, limits) {
  var capacity = isInteractive ? maximumConcurrency(limits) : backgroundCapacity(limits)
  return Math.max(0, Number(inFlight) || 0) < capacity
}

function nextWaitingIndex(priorities, inFlight, limits) {
  var list = Array.isArray(priorities) ? priorities : []
  for (var index = 0; index < list.length; index++)
    if (list[index] && canTransmit(true, inFlight, limits)) return index
  if (!canTransmit(false, inFlight, limits)) return -1
  for (var fallback = 0; fallback < list.length; fallback++)
    if (!list[fallback]) return fallback
  return -1
}

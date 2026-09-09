.pragma library

var MODES = ["active", "all", "locked"]
var DEFAULT = "active"

function normalize(value) {
  var mode = String(value || "").toLowerCase()
  if (mode === "primary") return "locked"
  return MODES.indexOf(mode) >= 0 ? mode : DEFAULT
}

function eligible(mode, screenName, context) {
  var name = String(screenName || "")
  var settings = context || {}
  if (name === "") return false
  switch (normalize(mode)) {
  case "all": return true
  case "locked": return name === String(settings.lock || "")
  default: return String(settings.openedOn || "") !== "" && name === String(settings.openedOn || "")
  }
}

function invocationName(mode, context) {
  var settings = context || {}
  switch (normalize(mode)) {
  case "all": return ""
  case "locked": return String(settings.lock || "")
  default: return String(settings.focused || "")
  }
}

function preferredName(mode, context) {
  var settings = context || {}
  if (String(settings.openedOn || "") !== "") return String(settings.openedOn || "")
  switch (normalize(mode)) {
  case "all": return String(settings.primary || "")
  case "locked": return String(settings.lock || "")
  default: return String(settings.focused || "")
  }
}

function choiceKey(mode, lock) {
  return normalize(mode) === "locked" ? "lock:" + String(lock || "") : normalize(mode)
}

function parseChoice(key) {
  var value = String(key || "")
  if (value.indexOf("lock:") === 0) return { mode: "locked", lock: value.substring(5) }
  return { mode: normalize(value), lock: "" }
}

function choices(screenNames) {
  var rows = [{ key: "active", label: "Active" }, { key: "all", label: "All" }]
  var names = Array.isArray(screenNames) ? screenNames : []
  for (var i = 0; i < names.length; i++) {
    var name = String(names[i] || "")
    if (name !== "") rows.push({ key: "lock:" + name, label: "Lock to " + name })
  }
  return rows
}

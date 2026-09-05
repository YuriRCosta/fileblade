.pragma library

var LOCAL_ACTIONS = ["copy-paths", "open-with"]

function transfersFocus(actionId, fileCount) {
  var action = String(actionId || "")
  if (!action) return false
  if (action === "open") return Math.max(0, Number(fileCount) || 0) > 0
  return LOCAL_ACTIONS.indexOf(action) < 0
}

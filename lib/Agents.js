.pragma library

var ALL_GLYPH = "󰚩"

var KNOWN = [
  { id: "claude-code", label: "Claude Code", svg: "claude-code.svg", color: "#D97757" },
  { id: "codex", label: "Codex", svg: "codex.svg", color: "" },
  { id: "opencode", label: "OpenCode", svg: "opencode.svg", color: "" },
  { id: "pi", label: "Pi", svg: "pi.svg", color: "" },
  { id: "copilot-cli", label: "GitHub Copilot CLI", glyph: "", color: "" },
  { id: "antigravity", label: "Google Antigravity", svg: "antigravity.svg", color: "#8B5CF6" }
]

function byId(id) {
  var wanted = String(id || "")
  for (var i = 0; i < KNOWN.length; i++) if (KNOWN[i].id === wanted) return KNOWN[i]
  return null
}

function ordered(ids) {
  var wanted = []
  var count = ids && ids.length !== undefined ? Number(ids.length) : 0
  for (var j = 0; j < count; j++) wanted.push(String(ids[j]))
  var result = []
  for (var i = 0; i < KNOWN.length; i++) if (wanted.indexOf(KNOWN[i].id) >= 0) result.push(KNOWN[i].id)
  return result
}

function label(id) {
  var agent = byId(id)
  return agent ? agent.label : String(id || "")
}

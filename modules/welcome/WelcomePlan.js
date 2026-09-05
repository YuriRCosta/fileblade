.pragma library

var HEADING = "Install agent extensions?"
var BODY = "FileBlade can be extended and customized.\n\nInstall these example extensions which show an overview of all your agent memory files, skills, MCPs, and hooks. You can also make your own extensions, or ask your agent to put an existing Omarchy plugin here."
var DISMISS = "Close and don't show this again"
var SOURCE_NOTICE = "Until the plugins are in the Omarchy plugin registry, they are installed from the public GitHub repos. Click each plugin to navigate to its source code."
var STATES = ["", "dismissed", "installed"]
var EXTENSIONS = [
  { id: "data-goblin.fileblade-memory", module: "data-goblin.fileblade-memory/memory", name: "Memory", glyph: "󰧑",
    url: "https://github.com/data-goblin/fileblade-memory.git" },
  { id: "data-goblin.fileblade-skills", module: "data-goblin.fileblade-skills/skills", name: "Skills", glyph: "󰖷",
    url: "https://github.com/data-goblin/fileblade-skills.git" },
  { id: "data-goblin.fileblade-mcp", module: "data-goblin.fileblade-mcp/mcp", name: "MCP", glyph: "", image: "../../assets/mcp-logo.svg",
    url: "https://github.com/data-goblin/fileblade-mcp.git" },
  { id: "data-goblin.fileblade-hooks", module: "data-goblin.fileblade-hooks/hooks", name: "Hooks", glyph: "󰛢",
    url: "https://github.com/data-goblin/fileblade-hooks.git" }
]

var PLACEMENTS = [
  { module: "data-goblin.fileblade-skills/skills", target: "top" },
  { module: "data-goblin.fileblade-mcp/mcp", target: "top" },
  { module: "data-goblin.fileblade-hooks/hooks", target: "top" },
  { module: "data-goblin.fileblade-memory/memory", target: "notes" }
]
var PLACEMENT_EDGE = "right"

function topModules() {
  return PLACEMENTS.filter(function(placement) { return placement.target === "top" }).map(function(placement) { return placement.module })
}

function topSlotIndex(slots) {
  var wanted = topModules()
  for (var i = 0; i < slots.length; i++) {
    var modules = slots[i] && slots[i].modules ? slots[i].modules : []
    for (var j = 0; j < modules.length; j++) {
      if (wanted.indexOf(String(modules[j].module || "")) >= 0) return i
    }
  }
  return -1
}

function unplaced(findModule) {
  return PLACEMENTS.filter(function(placement) { return !findModule(placement.module) })
}

function installCommand(extension) {
  return ["omarchy-plugin-add", String(extension.url), "--yes", "--enable"]
}

function pending(state) {
  return String(state || "") === ""
}

function extensionName(module) {
  for (var i = 0; i < EXTENSIONS.length; i++) {
    if (EXTENSIONS[i].module === module) return EXTENSIONS[i].name
  }
  return String(module || "")
}

function normalizeState(state) {
  var value = String(state || "")
  return STATES.indexOf(value) >= 0 ? value : ""
}

function missing(registryHas) {
  return EXTENSIONS.filter(function(extension) { return !registryHas(extension.module) })
}

function welcomeSlot() {
  return { id: "welcome", modules: [{ module: "welcome" }, { module: "notes" }], active: 0 }
}

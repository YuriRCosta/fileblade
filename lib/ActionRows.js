.pragma library

var CONTEXTS = ["file", "dir", "selection", "root", "none"]
var SOURCES = ["plugin", "user"]
var OUTPUTS = ["notice", "silent"]
var CONTROL_CHARACTERS = /[\u0000-\u001f\u007f-\u009f\u202a-\u202e\u2066-\u2069\u2028\u2029]/g
var MAXIMUM_ROWS = 64
var MAXIMUM_TARGETS = 256
var MAXIMUM_KEY_LENGTH = 200
var MAXIMUM_ID_LENGTH = 64
var MAXIMUM_PLUGIN_LENGTH = 128
var MAXIMUM_PATH_LENGTH = 4096
var MAXIMUM_PATH_DOCUMENT_LENGTH = 65536
var MAXIMUM_ENCODED_PATH_DOCUMENT_LENGTH = 87391
var MAXIMUM_TITLE_LENGTH = 64
var MAXIMUM_GLYPH_LENGTH = 16
var MAXIMUM_DESCRIPTION_LENGTH = 160
var MINIMUM_TIMEOUT = 1
var MAXIMUM_TIMEOUT = 900
var DEFAULT_TIMEOUT = 60
var DEFAULT_GLYPH = "󰑮"

function text(value, limit) {
  var raw = String(value === undefined || value === null ? "" : value)
  return raw.replace(CONTROL_CHARACTERS, "").slice(0, limit)
}

function firstLine(value, limit) {
  var raw = String(value === undefined || value === null ? "" : value)
  var breakIndex = raw.indexOf("\n")
  return text(breakIndex >= 0 ? raw.slice(0, breakIndex) : raw, limit).trim()
}

function contextList(raw) {
  var declared = Array.isArray(raw) ? raw : []
  var result = []
  for (var i = 0; i < declared.length && result.length < CONTEXTS.length; i++) {
    var name = typeof declared[i] === "string" ? declared[i] : ""
    if (CONTEXTS.indexOf(name) >= 0 && result.indexOf(name) < 0) result.push(name)
  }
  return result
}

function word(value, allowed, fallback) {
  var name = typeof value === "string" ? value : ""
  return allowed.indexOf(name) >= 0 ? name : fallback
}

function action(raw) {
  if (!raw || typeof raw !== "object" || Array.isArray(raw)) return null
  var key = text(raw.key, MAXIMUM_KEY_LENGTH).trim()
  var id = text(raw.id, MAXIMUM_ID_LENGTH).trim()
  var title = text(raw.title, MAXIMUM_TITLE_LENGTH).trim()
  var accepted = contextList(raw.contexts)
  if (key === "" || id === "" || title === "" || accepted.length === 0) return null
  var glyph = text(raw.glyph, MAXIMUM_GLYPH_LENGTH).trim()
  var timeout = Math.floor(Number(raw.timeout))
  return {
    key: key,
    id: id,
    source: word(raw.source, SOURCES, "plugin"),
    plugin: text(raw.plugin, MAXIMUM_PLUGIN_LENGTH),
    pluginRoot: text(raw.pluginRoot, MAXIMUM_PATH_LENGTH),
    title: title,
    glyph: glyph === "" ? DEFAULT_GLYPH : glyph,
    description: text(raw.description, MAXIMUM_DESCRIPTION_LENGTH).trim(),
    contexts: accepted,
    confirm: raw.confirm === true,
    detach: raw.detach === true,
    output: word(raw.output, OUTPUTS, "notice"),
    timeout: isFinite(timeout)
      ? Math.max(MINIMUM_TIMEOUT, Math.min(MAXIMUM_TIMEOUT, timeout)) : DEFAULT_TIMEOUT,
    program: text(raw.program, MAXIMUM_PATH_LENGTH)
  }
}

function normalize(rows) {
  var raw = Array.isArray(rows) ? rows : []
  var result = []
  for (var i = 0; i < raw.length && result.length < MAXIMUM_ROWS; i++) {
    var row = action(raw[i])
    if (row) result.push(row)
  }
  return result
}

function errors(rows) {
  var raw = Array.isArray(rows) ? rows : []
  var result = []
  for (var i = 0; i < raw.length && result.length < MAXIMUM_ROWS; i++) {
    var item = raw[i] && typeof raw[i] === "object" ? raw[i] : ({})
    var error = firstLine(item.error, MAXIMUM_DESCRIPTION_LENGTH)
    if (error !== "") result.push({ source: text(item.source, MAXIMUM_PLUGIN_LENGTH), error: error })
  }
  return result
}

function isDirectory(entry) {
  var value = entry && typeof entry === "object" ? entry : ({})
  return value.isDir === true || value.is_dir === true
}

function entryPath(entry) {
  if (typeof entry === "string") return entry
  var value = entry && typeof entry === "object" ? entry : ({})
  return String(value.path === undefined || value.path === null ? "" : value.path)
}

function applies(context, entries, root) {
  var list = Array.isArray(entries) ? entries : []
  switch (context) {
  case "file":
    return list.length === 1 && !isDirectory(list[0])
  case "dir":
    return list.length === 1 && isDirectory(list[0])
  case "selection":
    return list.length >= 1
  case "root":
    return String(root === undefined || root === null ? "" : root) !== ""
  default:
    return true
  }
}

function contextFor(entries, root) {
  for (var i = 0; i < CONTEXTS.length; i++)
    if (applies(CONTEXTS[i], entries, root)) return CONTEXTS[i]
  return "none"
}

function matches(row, context) {
  var accepted = row && Array.isArray(row.contexts) ? row.contexts : []
  return accepted.indexOf(String(context === undefined || context === null ? "" : context)) >= 0
}

function contextOf(row, entries, root) {
  for (var i = 0; i < CONTEXTS.length; i++)
    if (matches(row, CONTEXTS[i]) && applies(CONTEXTS[i], entries, root)) return CONTEXTS[i]
  return ""
}

function rowsFor(actions, entries, root) {
  var available = Array.isArray(actions) ? actions : []
  var result = []
  for (var i = 0; i < available.length && result.length < MAXIMUM_ROWS; i++)
    if (contextOf(available[i], entries, root) !== "") result.push(available[i])
  return result
}

function targetsFor(context, entries, root) {
  var list = Array.isArray(entries) ? entries : []
  if (context === "file" || context === "dir") {
    var only = entryPath(list[0])
    return only === "" ? [] : [only]
  }
  if (context !== "selection") return []
  if (list.length > MAXIMUM_TARGETS) return []
  var result = []
  for (var i = 0; i < list.length; i++) {
    var path = entryPath(list[i])
    if (path !== "") result.push(path)
  }
  return result
}

function targetCount(context, entries, root) {
  if (context === "root") return String(root === undefined || root === null ? "" : root) === "" ? 0 : 1
  if (context === "selection") return Array.isArray(entries) ? entries.length : 0
  return targetsFor(context, entries, root).length
}

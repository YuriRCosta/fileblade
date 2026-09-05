.pragma library

var MODULE_ID_PATTERN = /^[A-Za-z0-9][A-Za-z0-9._-]*(\/[A-Za-z0-9][A-Za-z0-9._-]*)?$/
var BARE_ID_PATTERN = /^[A-Za-z0-9][A-Za-z0-9._-]*$/
var CATEGORY_PATTERN = /^[A-Za-z][A-Za-z0-9 ]{0,31}$/
var SETTING_KEY_PATTERN = /^[A-Za-z_][A-Za-z0-9_]{0,63}$/
var CONTROL_CHARACTERS = /[\u0000-\u001f\u007f-\u009f\u202a-\u202e\u2066-\u2069]/g
var MAXIMUM_ID_LENGTH = 128
var MAXIMUM_SCHEMA_ROWS = 32
var MAXIMUM_OPTIONS = 32
var MAXIMUM_LABEL_LENGTH = 64
var MAXIMUM_DESCRIPTION_LENGTH = 160
var MAXIMUM_PLACEHOLDER_LENGTH = 64
var DEFAULT_TEXT_LENGTH = 256
var MAXIMUM_TEXT_LENGTH = 4096
var NUMERIC_BOUND = 1e9
var MINIMUM_NUMBER_STEP = 1e-9
var DEFAULT_CATEGORY = "Module"
var PLUGIN_CATEGORY = "Plugin"
var RESERVED_SETTING_KEYS = lookup(Object.getOwnPropertyNames(Object.prototype).concat(["id", "__proto__"]))
var SETTING_TYPES = lookup(["string", "integer", "number", "boolean", "enum", "path"])

function lookup(names) {
  var table = Object.create(null)
  for (var i = 0; i < names.length; i++) table[names[i]] = true
  return table
}

function boundedText(value, fallback, limit) {
  var text = String(value === undefined || value === null ? fallback : value)
  return text.replace(CONTROL_CHARACTERS, "").slice(0, limit)
}

function textField(value, fallback, limit) {
  return boundedText(typeof value === "object" ? fallback : value, fallback, limit)
}

function safeId(value, limit) {
  var text = boundedText(value, "", Math.max(0, Number(limit) || 0) + 1).trim()
  return BARE_ID_PATTERN.test(text) && text.length <= limit ? text : ""
}

function category(raw, source) {
  var fallback = String(source || "").indexOf("plugin:") === 0 ? PLUGIN_CATEGORY : DEFAULT_CATEGORY
  if (typeof raw !== "string") return fallback
  var text = boundedText(raw, "", MAXIMUM_LABEL_LENGTH).trim()
  return CATEGORY_PATTERN.test(text) ? text : fallback
}

function categoryRank(value) {
  if (value === DEFAULT_CATEGORY) return 0
  if (value === PLUGIN_CATEGORY) return 2
  return 1
}

function categoryOrder(left, right) {
  var leftText = String(left || "")
  var rightText = String(right || "")
  var difference = categoryRank(leftText) - categoryRank(rightText)
  if (difference !== 0) return difference
  return leftText.localeCompare(rightText)
}

function dirName(moduleId) {
  var text = String(moduleId === undefined || moduleId === null ? "" : moduleId)
  if (text.length > MAXIMUM_ID_LENGTH || !MODULE_ID_PATTERN.test(text)) return ""
  return text.replace("/", "+")
}

function finiteNumber(value, fallback) {
  var number = typeof value === "boolean" ? NaN : Number(value)
  return typeof value !== "object" && value !== "" && isFinite(number) ? number : fallback
}

function optionValue(option) {
  if (typeof option === "string") return boundedText(option, "", MAXIMUM_LABEL_LENGTH)
  if (typeof option === "number" && isFinite(option)) return String(option)
  if (!option || typeof option !== "object" || Array.isArray(option)) return null
  if (option.value === undefined || option.value === null) return null
  return boundedText(typeof option.value === "object" ? "" : option.value, "", MAXIMUM_LABEL_LENGTH)
}

function normalizedOptions(rawOptions) {
  if (!Array.isArray(rawOptions)) return []
  var result = []
  var seen = Object.create(null)
  for (var i = 0; i < rawOptions.length && result.length < MAXIMUM_OPTIONS; i++) {
    var value = optionValue(rawOptions[i])
    if (value === null || value === "" || seen[value]) continue
    var label = rawOptions[i] && typeof rawOptions[i] === "object" && rawOptions[i].label !== undefined
      ? textField(rawOptions[i].label, value, MAXIMUM_LABEL_LENGTH) : value
    seen[value] = true
    result.push({ value: value, label: label === "" ? value : label })
  }
  return result
}

function optionIndex(row, value) {
  if (value === undefined || value === null || typeof value === "object") return -1
  var text = String(value)
  for (var i = 0; i < row.options.length; i++)
    if (row.options[i].value === text) return i
  return -1
}

function coerceDefault(row, value) {
  if (value === undefined || value === null) return undefined
  switch (row.type) {
  case "string":
  case "path":
    return typeof value === "object" ? undefined : boundedText(value, "", row.maxLength)
  case "integer": {
    var integer = finiteNumber(value, NaN)
    return isNaN(integer) ? undefined : Math.max(row.min, Math.min(row.max, Math.floor(integer)))
  }
  case "number": {
    var number = finiteNumber(value, NaN)
    return isNaN(number) ? undefined : Math.max(row.min, Math.min(row.max, number))
  }
  case "boolean":
    if (value === "true") return true
    if (value === "false") return false
    return typeof value === "object" ? undefined : !!value
  case "enum": {
    var index = optionIndex(row, value)
    return index < 0 ? undefined : row.options[index].value
  }
  }
  return undefined
}

function typeZero(row) {
  switch (row.type) {
  case "integer":
  case "number":
    return Math.max(row.min, Math.min(row.max, 0))
  case "boolean":
    return false
  case "enum":
    return row.options[0].value
  }
  return ""
}

function numericBounds(row, raw, integer) {
  var lowest = -NUMERIC_BOUND
  var highest = NUMERIC_BOUND
  var min = Math.max(lowest, Math.min(highest, finiteNumber(raw.min, lowest)))
  var max = Math.max(lowest, Math.min(highest, finiteNumber(raw.max, highest)))
  if (integer) {
    min = Math.ceil(min)
    max = Math.floor(max)
  }
  if (min > max) {
    min = lowest
    max = highest
  }
  row.min = min
  row.max = max
  var step = finiteNumber(raw.step, integer ? 1 : 0.1)
  if (integer) step = Math.max(1, Math.floor(step))
  else if (!(step >= MINIMUM_NUMBER_STEP)) step = 0.1
  var span = max - min
  row.step = span > 0 && step > span ? span : step
}

function settingsRow(raw, seen) {
  if (!raw || typeof raw !== "object" || Array.isArray(raw)) return null
  var key = typeof raw.key === "string" ? raw.key : ""
  if (!SETTING_KEY_PATTERN.test(key) || RESERVED_SETTING_KEYS[key] || seen[key]) return null
  var type = typeof raw.type === "string" ? raw.type : ""
  if (!SETTING_TYPES[type]) return null
  var row = {
    key: key,
    type: type,
    label: textField(raw.label, key, MAXIMUM_LABEL_LENGTH).trim() || key,
    description: textField(raw.description, "", MAXIMUM_DESCRIPTION_LENGTH).trim(),
    group: textField(raw.group, "", MAXIMUM_LABEL_LENGTH).trim()
  }
  if (type === "string" || type === "path") {
    var length = Math.floor(finiteNumber(raw.maxLength, DEFAULT_TEXT_LENGTH))
    row.maxLength = Math.max(1, Math.min(MAXIMUM_TEXT_LENGTH, length))
    row.placeholder = textField(raw.placeholder, "", MAXIMUM_PLACEHOLDER_LENGTH)
  } else if (type === "integer" || type === "number") {
    numericBounds(row, raw, type === "integer")
  } else if (type === "enum") {
    row.options = normalizedOptions(raw.options)
    if (row.options.length === 0) return null
  }
  return row
}

function settingsSpec(raw) {
  var schema = []
  var defaults = ({})
  if (!raw || typeof raw !== "object" || Array.isArray(raw)) return { schema: schema, defaults: defaults }
  var declared = raw.defaults && typeof raw.defaults === "object" && !Array.isArray(raw.defaults) ? raw.defaults : ({})
  var rows = Array.isArray(raw.schema) ? raw.schema : []
  var seen = Object.create(null)
  for (var i = 0; i < rows.length && schema.length < MAXIMUM_SCHEMA_ROWS; i++) {
    var row = settingsRow(rows[i], seen)
    if (!row) continue
    seen[row.key] = true
    var value = coerceDefault(row, rows[i].defaultValue)
    if (value === undefined && Object.prototype.hasOwnProperty.call(declared, row.key))
      value = coerceDefault(row, declared[row.key])
    row.defaultValue = value === undefined ? typeZero(row) : value
    defaults[row.key] = row.defaultValue
    schema.push(row)
  }
  return { schema: schema, defaults: defaults }
}

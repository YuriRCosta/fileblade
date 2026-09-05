.pragma library
.import "Definitions.js" as Definitions

var MAXIMUM_DECIMALS = 12
var MAXIMUM_FIELD_LENGTH = 32
var INLINE_OPTION_LIMIT = 4

function row(schema, key) {
  if (!Array.isArray(schema)) return null
  var name = String(key === undefined || key === null ? "" : key)
  for (var i = 0; i < schema.length; i++)
    if (schema[i] && typeof schema[i] === "object" && schema[i].key === name) return schema[i]
  return null
}

function coerce(row, raw) {
  if (!row || typeof row !== "object") return undefined
  return Definitions.coerceDefault(row, raw)
}

function effective(row, raw) {
  var value = coerce(row, raw)
  return value === undefined ? row.defaultValue : value
}

function storedValue(values, key) {
  if (!values || typeof values !== "object") return undefined
  return Object.prototype.hasOwnProperty.call(values, key) ? values[key] : undefined
}

function numeric(row) {
  return !!row && (row.type === "integer" || row.type === "number")
}

function decimals(step) {
  var text = String(step)
  var exponent = text.indexOf("e-")
  if (exponent >= 0) {
    var mantissa = text.slice(0, exponent)
    var point = mantissa.indexOf(".")
    var fraction = point < 0 ? 0 : mantissa.length - point - 1
    return Math.min(MAXIMUM_DECIMALS, Number(text.slice(exponent + 2)) + fraction)
  }
  var dot = text.indexOf(".")
  return dot < 0 ? 0 : Math.min(MAXIMUM_DECIMALS, text.length - dot - 1)
}

function formatValue(row, value) {
  if (!numeric(row)) return String(value === undefined || value === null ? "" : value)
  var number = effective(row, value)
  if (row.type === "integer") return String(Math.floor(number))
  return String(Number(number.toFixed(decimals(row.step))))
}

function stepValue(row, current, direction) {
  var base = effective(row, current)
  var next = base + (direction < 0 ? -1 : 1) * row.step
  var rounded = row.type === "integer" ? Math.round(next) : Number(next.toFixed(decimals(row.step)))
  return Math.max(row.min, Math.min(row.max, rounded))
}

function rangeText(row) {
  if (!numeric(row)) return ""
  var lower = row.min > -Definitions.NUMERIC_BOUND
  var upper = row.max < Definitions.NUMERIC_BOUND
  if (lower && upper) return formatValue(row, row.min) + " to " + formatValue(row, row.max)
  if (lower) return "from " + formatValue(row, row.min)
  if (upper) return "up to " + formatValue(row, row.max)
  return ""
}

function optionLabel(row, value) {
  if (!row || !Array.isArray(row.options)) return ""
  var index = Definitions.optionIndex(row, effective(row, value))
  return index < 0 ? "" : row.options[index].label
}

function choiceOptions(row) {
  if (!row || !Array.isArray(row.options)) return []
  return row.options.map(function(option) { return { key: option.value, label: option.label } })
}

function popupRows(row, value) {
  if (!row || !Array.isArray(row.options)) return []
  var current = effective(row, value)
  return row.options.map(function(option) {
    return { key: option.value, label: option.label, checked: option.value === current }
  })
}

function inlineChoice(row) {
  return !!row && row.type === "enum" && Array.isArray(row.options) && row.options.length <= INLINE_OPTION_LIMIT
}

function rowModel(schema, values) {
  var rows = Array.isArray(schema) ? schema : []
  var result = []
  for (var i = 0; i < rows.length && result.length < Definitions.MAXIMUM_SCHEMA_ROWS; i++) {
    var source = rows[i]
    if (!source || typeof source !== "object" || typeof source.key !== "string" || !source.key) continue
    var entry = {
      key: source.key,
      type: String(source.type),
      label: String(source.label === undefined ? source.key : source.label),
      description: String(source.description === undefined ? "" : source.description),
      group: String(source.group === undefined ? "" : source.group),
      defaultValue: source.defaultValue,
      value: effective(source, storedValue(values, source.key)),
      detail: rangeText(source),
      options: Array.isArray(source.options) ? source.options : [],
      min: source.min,
      max: source.max,
      step: source.step,
      maxLength: source.maxLength,
      placeholder: String(source.placeholder === undefined ? "" : source.placeholder)
    }
    result.push(entry)
  }
  return result
}

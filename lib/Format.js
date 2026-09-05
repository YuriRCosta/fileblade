.pragma library

var DASH = "—"

function significant(value, units, base, separator) {
  if (value === null || value === undefined || value === "") return DASH
  var number = Number(value)
  if (!isFinite(number) || number < 0) return DASH
  if (number < 999.5) return String(Math.round(number)) + (units.length && separator ? separator + units[0] : "")
  var unit = 0
  while (number >= 999.5 && unit < units.length - 1) {
    number /= base
    unit++
  }
  return number.toPrecision(3) + separator + units[unit]
}

function compact(value) {
  return significant(value, ["", "k", "m", "b", "t"], 1000, "")
}

function bytes(value) {
  return significant(value, ["B", "KB", "MB", "GB", "TB"], 1024, " ")
}

function isoDate(date) {
  var month = String(date.getMonth() + 1)
  var day = String(date.getDate())
  return date.getFullYear() + "-" + (month.length < 2 ? "0" + month : month) + "-" + (day.length < 2 ? "0" + day : day)
}

function shiftedDate(amount, unit) {
  var date = new Date()
  var steps = { d: 1, w: 7 }
  if (unit === "m") date.setMonth(date.getMonth() - amount)
  else if (unit === "y") date.setFullYear(date.getFullYear() - amount)
  else date.setDate(date.getDate() - amount * steps[unit])
  return isoDate(date)
}

function parseDate(text) {
  var raw = String(text || "").trim().toLowerCase()
  if (raw === "") return ""
  if (raw === "today") return shiftedDate(0, "d")
  if (raw === "yesterday") return shiftedDate(1, "d")
  var relative = raw.match(/^(\d+)\s*([dwmy])$/)
  if (relative) return shiftedDate(Number(relative[1]), relative[2])
  var iso = raw.match(/^(\d{4})(?:-(\d{1,2}))?(?:-(\d{1,2}))?/)
  if (!iso) return ""
  var month = Number(iso[2] || "1")
  var day = Number(iso[3] || "1")
  var date = new Date(Number(iso[1]), month - 1, day)
  var valid = date.getFullYear() === Number(iso[1]) && date.getMonth() === month - 1 && date.getDate() === day
  return valid ? isoDate(date) : ""
}

function parseNumber(text) {
  var raw = String(text || "").trim().toLowerCase().replace(/,/g, "")
  var match = raw.match(/^(\d+(?:\.\d+)?)\s*([kmbt]?)$/)
  if (!match) return NaN
  var scale = { "": 1, k: 1e3, m: 1e6, b: 1e9, t: 1e12 }
  return Number(match[1]) * scale[match[2]]
}

function metricKind(option) {
  var key = option && typeof option === "object" ? String(option.key || "") : String(option || "")
  var declared = option && typeof option === "object" ? String(option.kind || "") : ""
  if (declared) return declared
  if (key === "agents") return "agents"
  if (key === "updated" || key === "created") return "date"
  if (["tokens", "characters", "words", "bytes"].indexOf(key) >= 0) return "number"
  return "text"
}

var TEXT_METRIC_KEYS = ["tokens", "characters", "words", "bytes"]

var METRIC_OPTIONS = {
  off: { label: "Off", shortLabel: "OFF", kind: "text" },
  agents: { label: "Agents", shortLabel: "AGENTS", kind: "agents" },
  status: { label: "Status", shortLabel: "STATUS", kind: "text" },
  updated: { label: "Updated", shortLabel: "UPDATED", kind: "date" },
  created: { label: "Created", shortLabel: "CREATED", kind: "date" },
  tokens: { label: "Tokens (estimated)", shortLabel: "TOKENS", kind: "number" },
  characters: { label: "Characters", shortLabel: "CHARACTERS", kind: "number" },
  words: { label: "Words", shortLabel: "WORDS", kind: "number" },
  bytes: { label: "Size", shortLabel: "SIZE", kind: "number" },
  summary: { label: "Summary", shortLabel: "SUMMARY", kind: "text" }
}

function estimateTokens(byteLength) {
  var number = Number(byteLength)
  if (!isFinite(number) || number < 0) return null
  return Math.ceil(number / 4)
}

function metricOption(spec) {
  var given = spec && typeof spec === "object" ? spec : { key: String(spec || "") }
  var key = String(given.key || "")
  var standard = METRIC_OPTIONS[key] || {}
  var label = given.label !== undefined ? String(given.label) : (standard.label || (key.charAt(0).toUpperCase() + key.slice(1)))
  var result = {
    key: key,
    label: label,
    shortLabel: given.shortLabel !== undefined ? String(given.shortLabel) : (standard.shortLabel || label.toUpperCase()),
    kind: given.kind !== undefined ? String(given.kind) : (standard.kind || metricKind(key))
  }
  for (var name in given) if (!(name in result)) result[name] = given[name]
  return result
}

function metricOptions(specs) {
  var result = []
  var count = specs && specs.length !== undefined ? Number(specs.length) : 0
  for (var i = 0; i < count; i++) result.push(metricOption(specs[i]))
  return result
}

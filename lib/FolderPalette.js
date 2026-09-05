.pragma library

var SHIFT = { red: -25, yellow: 18, green: 40 }
var KEYS = ["red", "orange", "yellow", "green", "cyan", "blue", "magenta", "muted"]

function parseUser(text) {
  var document = JSON.parse(String(text || "{}"))
  if (!document || typeof document !== "object" || Array.isArray(document)) throw new Error("colors.json must be an object")
  var folder = document.folder
  if (folder === undefined) return {}
  if (!folder || typeof folder !== "object" || Array.isArray(folder)) throw new Error("\"folder\" must be an object of colour names to hex values")
  var out = {}
  for (var i = 0; i < KEYS.length; i++) {
    var value = folder[KEYS[i]]
    if (value === undefined) continue
    if (typeof value !== "string" || !/^#[0-9a-fA-F]{6}$/.test(value)) throw new Error("\"" + KEYS[i] + "\" must be a six-digit hex colour, got " + JSON.stringify(value))
    out[KEYS[i]] = value.toLowerCase()
  }
  return out
}

function distinct(name, hex) {
  var degrees = SHIFT[String(name || "")]
  if (!degrees || !/^#[0-9a-fA-F]{6}$/.test(String(hex || ""))) return hex
  var hsl = toHsl(hex)
  hsl.h = (hsl.h + degrees + 360) % 360
  return toHex(hsl)
}

function toHsl(hex) {
  var r = parseInt(hex.slice(1, 3), 16) / 255
  var g = parseInt(hex.slice(3, 5), 16) / 255
  var b = parseInt(hex.slice(5, 7), 16) / 255
  var max = Math.max(r, g, b), min = Math.min(r, g, b)
  var l = (max + min) / 2
  var d = max - min
  if (d === 0) return { h: 0, s: 0, l: l }
  var s = d / (1 - Math.abs(2 * l - 1))
  var h
  if (max === r) h = ((g - b) / d) % 6
  else if (max === g) h = (b - r) / d + 2
  else h = (r - g) / d + 4
  return { h: (h * 60 + 360) % 360, s: s, l: l }
}

function toHex(hsl) {
  var c = (1 - Math.abs(2 * hsl.l - 1)) * hsl.s
  var x = c * (1 - Math.abs((hsl.h / 60) % 2 - 1))
  var m = hsl.l - c / 2
  var sector = Math.floor(hsl.h / 60) % 6
  var rgb = [[c, x, 0], [x, c, 0], [0, c, x], [0, x, c], [x, 0, c], [c, 0, x]][sector]
  var out = "#"
  for (var i = 0; i < 3; i++) {
    var v = Math.round((rgb[i] + m) * 255)
    out += (v < 16 ? "0" : "") + Math.max(0, Math.min(255, v)).toString(16)
  }
  return out
}

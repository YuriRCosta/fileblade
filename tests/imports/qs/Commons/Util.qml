pragma Singleton
import QtQuick
QtObject {
  function alpha(color, opacity) {
    if (!color) return Qt.rgba(0, 0, 0, 0)
    var value = Math.max(0, Math.min(1, Number(opacity)))
    var resolved = typeof color === "string" ? Qt.color(color) : color
    return Qt.rgba(resolved.r, resolved.g, resolved.b, isNaN(value) ? 1 : value)
  }

  function fileUrl(path) {
    if (!path) return ""
    return "file://" + String(path).split("/").map(encodeURIComponent).join("/")
  }
}

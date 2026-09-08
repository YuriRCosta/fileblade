pragma Singleton
import QtQuick
QtObject {
  function alpha(color, opacity) { return Qt.rgba(color.r, color.g, color.b, opacity) }

  function fileUrl(path) {
    if (!path) return ""
    return "file://" + String(path).split("/").map(encodeURIComponent).join("/")
  }
}

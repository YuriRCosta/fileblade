pragma Singleton
import QtQml
QtObject {
  property real scale: 1
  property real cornerRadius: 4 * scale
  property var font: ({ family: "monospace", title: 18 * scale, body: 14 * scale, bodySmall: 12 * scale, caption: 11 * scale })
  function space(value) { return value * scale }
}

pragma Singleton
import QtQml
QtObject {
  property int cornerRadius: 0
  property var font: ({ family: "monospace", title: 14, body: 12, bodySmall: 11, caption: 10 })
  function space(value) { return Math.max(0, Math.round(Number(value) || 0)) }
}

import QtQuick

Item {
  required property var context
  property bool flushOnClose: false
  readonly property var provider: context.providerService
  readonly property bool active: context.bladeOpen
  onActiveChanged: if (!active && flushOnClose) context.state.set("flush", true)
  Component.onCompleted: provider.attach(context)
  Component.onDestruction: {
    provider.detach(context)
    context.state.set("retiring", true)
    context.requestFocus("")
  }
}

import QtQuick

Item {
  required property var service
  readonly property string excludePattern: ""
  readonly property bool filtered: false
  readonly property bool stopPending: false
  readonly property int handoffCount: 0
  readonly property var desiredCommand: []
}

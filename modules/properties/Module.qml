import QtQuick
import "../../panes"

FocusScope {
  id: module

  required property var context

  readonly property var controller: context.service("files")
  readonly property string title: "Properties"
  readonly property var shortcuts: [
    {
      title: "Properties",
      items: [
        { shortcut: "j/k  ↓/↑", text: "Scroll" },
        { shortcut: "g/G  Home/End", text: "Top / bottom" },
        { shortcut: "Ctrl+D/U", text: "Page down / up" },
        { shortcut: "Esc", text: "Close the blade" },
        { shortcut: "h  ←", text: "Back to the tree" },
        { shortcut: "/", text: "Search" },
        { shortcut: "Enter/o", text: "Open" },
        { shortcut: "Shift+Enter", text: "Open with" },
        { shortcut: "e", text: "Edit in LazyVim" },
        { shortcut: "r", text: "Reveal in file manager" },
        { shortcut: "m  Menu", text: "Actions" },
        { shortcut: "F2", text: "Rename" },
        { shortcut: "Delete", text: "Trash" },
        { shortcut: "Ctrl+N  Ctrl+Shift+N", text: "New file / folder" },
        { shortcut: "Ctrl+C/X/V", text: "Copy / cut / paste" },
        { shortcut: "u  Ctrl+Z/Y", text: "Undo / redo" },
        { shortcut: "Backspace  Alt+←/→", text: "Back / forward" },
        { shortcut: "q", text: "Close blade" }
      ]
    }
  ]

  function takeFocus(part) {
    pane.forcePaneFocus()
  }

  PropertiesPane {
    id: pane
    anchors.fill: parent
    controller: module.controller
    hostWindow: module.context.hostWindow
    context: module.context
    focusEnabled: module.context.bladeOpen
  }
}

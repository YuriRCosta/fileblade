import QtQuick

Item {
  id: service

  property var shell: null
  property var manifest: null
  property var pluginRegistry: null

  readonly property string pluginDir: manifest && manifest.__sourceDir ? String(manifest.__sourceDir) : ""
  readonly property int viewCount: observers.length
  property var observers: []

  function attach(context) {
    if (!context || observers.indexOf(context) >= 0) return
    observers = observers.concat([context])
  }

  function detach(context) {
    observers = observers.filter(function(value) { return value !== context })
  }

  Loader {
    active: !!service.pluginRegistry
    source: "HostGuard.qml"
    onLoaded: {
      item.pluginRegistry = Qt.binding(function() { return service.pluginRegistry })
      item.pluginId = Qt.binding(function() { return service.manifest ? String(service.manifest.id) : "" })
    }
  }
}

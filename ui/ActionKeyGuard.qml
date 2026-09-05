import QtQuick

QtObject {
  id: guard
  property bool active: true
  property var shared: null
  property Item releaseRoot: null
  property var held: ({})
  property var pendingRelease: ({})

  onActiveChanged: if (!active) reset()

  // Unmarked compositor repeats can include a same-turn release/press pair.
  function isRepeat(event) {
    if (shared) return shared.isRepeat(event)
    if (event.isAutoRepeat) return true
    var key = String(event.key)
    delete pendingRelease[key]
    if (held[key]) return true
    held[key] = true
    return false
  }

  function release(event) {
    if (shared) { shared.release(event); return }
    if (event.isAutoRepeat) return
    pendingRelease[String(event.key)] = true
    releaseTimer.restart()
  }

  function reset() {
    releaseTimer.stop()
    held = ({})
    pendingRelease = ({})
  }

  property Timer releaseTimer: Timer {
    interval: 0
    onTriggered: {
      for (var key in guard.pendingRelease) delete guard.held[key]
      guard.pendingRelease = ({})
    }
  }

  property Connections rootReleases: Connections {
    target: guard.releaseRoot ? guard.releaseRoot.Keys : null
    function onReleased(event) { guard.release(event) }
  }
}

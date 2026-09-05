import QtQuick

Item {
  id: expansion

  property bool active: true
  property bool loading: false
  property var revision: null
  property int rowCount: 0
  property var nextEntry: function() { return null }
  property var expandEntry: function(entry) {}
  property int maximumSteps: 256
  property int maximumRows: 20000
  property int maximumDepth: 64
  property bool running: false
  property int steps: 0
  property string error: ""

  function start() {
    stop(true)
    if (!active) return
    steps = 0
    running = true
    schedule()
  }

  function stop(clearError) {
    running = false
    advanceTimer.stop()
    if (clearError) error = ""
  }

  function schedule() {
    if (running && active) advanceTimer.restart()
  }

  function advance() {
    if (!running || !active) return
    var entry = nextEntry()
    if (!entry) {
      if (!loading) stop(false)
      return
    }
    if (steps >= maximumSteps || rowCount >= maximumRows || entry.depth > maximumDepth) {
      error = "Recursive expansion limit reached; select a smaller folder to continue"
      stop(false)
      return
    }
    steps++
    expandEntry(entry)
    schedule()
  }

  onActiveChanged: if (!active) stop(false)
  onRevisionChanged: schedule()
  onLoadingChanged: schedule()
  Timer { id: advanceTimer; interval: 0; onTriggered: expansion.advance() }
}

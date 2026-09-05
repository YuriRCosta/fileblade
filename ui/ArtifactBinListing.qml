import QtQuick

Item {
  id: listing
  visible: false
  required property var service
  property string module: ""
  property bool active: true
  property var rows: []
  property var request: null
  property int generation: 0
  property bool stopping: false
  readonly property bool ready: !stopping && active && !!service && module !== ""

  function cancel() {
    var previous = request
    request = null
    if (previous) previous.service.cancelBackendRequest(previous.id, previous.generation, true)
  }

  function suspend() {
    reload.stop()
    cancel()
  }

  function refresh() {
    suspend()
    if (ready) reload.restart()
  }

  function invalidate() {
    rows = []
    refresh()
  }

  function start() {
    suspend()
    if (!ready) return
    var read = { id: "", generation: ++generation, service: service }
    request = read
    read.id = service.backendRequest("bin-list", ["--module", module], read.generation, function(response) {
      if (listing.request !== read) return
      listing.request = null
      listing.rows = response && response.ok === true && Array.isArray(response.items) ? response.items : []
    })
  }

  onModuleChanged: invalidate()
  onServiceChanged: invalidate()
  onActiveChanged: refresh()
  Component.onCompleted: refresh()
  Component.onDestruction: { stopping = true; suspend() }

  Timer { id: reload; interval: 0; onTriggered: listing.start() }
}

import QtQuick

QtObject {
  id: catalog

  property var service: null
  property int generation: 0
  property var providers: []
  property string activation: "unknown"
  property string error: ""
  property bool loading: false
  readonly property int maximumProviders: 128

  signal refreshed()

  function accepted(response) {
    if (!response || response.ok !== true || !Array.isArray(response.providers)) return null
    var rows = []
    for (var i = 0; i < response.providers.length && rows.length < maximumProviders; i++) {
      var row = response.providers[i]
      if (!row || typeof row !== "object") continue
      var id = String(row.id || "")
      var dir = String(row.dir || "")
      if (!id || !dir || !row.manifest || typeof row.manifest !== "object") continue
      rows.push({ id: id, dir: dir, manifest: row.manifest, enabled: row.enabled === true })
    }
    return rows
  }

  function refresh() {
    if (!service || loading) return false
    loading = true
    service.backendRequest("plugin-catalog", [], catalog.generation, function(response) {
      catalog.loading = false
      var rows = catalog.accepted(response)
      if (rows === null) {
        catalog.error = String(response && (response.message || response.error) || "The installed extensions could not be read")
        return
      }
      catalog.error = ""
      catalog.activation = String(response.activation || "unknown")
      catalog.providers = rows
      catalog.refreshed()
    })
    return true
  }
}

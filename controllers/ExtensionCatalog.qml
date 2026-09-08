import QtQuick

QtObject {
  id: catalog

  property var service: null
  property var watchPaths: []
  property int generation: 0
  property var providers: []
  property string activation: "unknown"
  property string error: ""
  property bool loading: false
  property double checkedAt: 0
  property string watchRequestId: ""
  readonly property int maximumProviders: 128
  readonly property int minimumIntervalMs: 2000
  readonly property int maximumAgeMs: 5000

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

  function withoutAuthority(rows) {
    var invalidated = []
    for (var i = 0; i < rows.length; i++)
      invalidated.push({ id: rows[i].id, dir: rows[i].dir, manifest: rows[i].manifest, enabled: false })
    return invalidated
  }

  function invalidate(message) {
    catalog.error = String(message || "The installed extensions could not be read")
    catalog.activation = "unknown"
    var invalidated = withoutAuthority(Array.isArray(providers) ? providers : [])
    catalog.providers = invalidated
    catalog.refreshed()
  }

  function stale() {
    return checkedAt <= 0 || Date.now() - checkedAt >= maximumAgeMs
  }

  function refreshIfStale() {
    return stale() ? refresh() : false
  }

  function refresh() {
    if (!service || loading) return false
    if (checkedAt > 0 && Date.now() - checkedAt < minimumIntervalMs) return false
    loading = true
    service.backendRequest("plugin-catalog", [], catalog.generation, function(response) {
      catalog.loading = false
      catalog.checkedAt = Date.now()
      var rows = catalog.accepted(response)
      if (rows === null) {
        catalog.invalidate(response && (response.message || response.error))
        return
      }
      catalog.error = ""
      catalog.activation = String(response.activation || "unknown")
      catalog.providers = catalog.activation === "known" ? rows : catalog.withoutAuthority(rows)
      catalog.refreshed()
    })
    return true
  }

  function watch() {
    if (!service || watchRequestId || !Array.isArray(watchPaths) || watchPaths.length === 0) return false
    generation++
    var current = generation
    watchRequestId = service.backendSubscribe(watchPaths.slice(), current, function() {
      if (current !== catalog.generation) return
      catalog.checkedAt = 0
      catalog.refresh()
    }, function() {
      if (current !== catalog.generation) return
      catalog.refresh()
    }, function() {
      if (current !== catalog.generation) return
      catalog.watchRequestId = ""
    })
    return watchRequestId !== ""
  }

  function unwatch() {
    if (!service || !watchRequestId) return
    service.cancelBackendRequest(watchRequestId, generation, true)
    watchRequestId = ""
    generation++
  }
}

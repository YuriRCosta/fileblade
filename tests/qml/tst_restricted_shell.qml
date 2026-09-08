import QtQuick
import QtTest
import "../../blades" as Blades
import "../../controllers" as Controllers

TestCase {
  id: testCase
  name: "RestrictedShellExtensions"

  QtObject {
    id: selfScopedRegistry
    property var installedPlugins: ({ "data-goblin.fileblade": { id: "data-goblin.fileblade" } })
    property var enabledIds: ["data-goblin.fileblade"]
    function isEnabled(id) { return enabledIds.indexOf(String(id)) >= 0 }
  }

  QtObject {
    id: fakeService
    property var lastRequest: null
    property var reply: null
    property var watchEvent: null
    property var watchPaths: []
    function backendRequest(name, args, generation, callback) {
      lastRequest = { name: name, args: args }
      if (callback) callback(reply)
      return "request-1"
    }
    function backendSubscribe(paths, generation, event, ready, closed) {
      watchPaths = paths
      watchEvent = event
      return "watch-1"
    }
    function cancelBackendRequest(id, generation, discard) { watchEvent = null }
    function emitWatchEvent() { if (watchEvent) watchEvent({ path: "/plugins" }) }
  }

  property var companionManifest: ({
    id: "acme.fileblade-weather",
    extensions: {
      "data-goblin.fileblade/blade": [{
        hostContract: 2,
        id: "weather",
        name: "Weather",
        entry: "blades/Module.qml",
        provider: "Provider.qml"
      }]
    }
  })

  Blades.BladeRegistry {
    id: registry
    socketId: "data-goblin.fileblade/blade"
    pluginDir: "/opt/fileblade"
    contractVersion: 2
    pluginRegistry: selfScopedRegistry
  }

  Controllers.ExtensionCatalog {
    id: catalog
    service: fakeService
  }

  Controllers.ExtensionProviders {
    id: providers
    inventoryUrl: "file:///opt/fileblade/ui/ArtifactInventory.qml"
    files: fakeService
  }

  function test_a_self_scoped_registry_hides_every_companion() {
    registry.catalogProviders = []
    var hidden = registry.manifestModules()
    compare(Object.keys(hidden).length, 0)
  }

  function test_the_catalog_restores_companion_modules_and_their_source() {
    registry.catalogProviders = [{
      id: "acme.fileblade-weather",
      dir: "/home/tester/.config/omarchy/plugins/acme.fileblade-weather",
      manifest: companionManifest,
      enabled: true
    }]
    var modules = registry.manifestModules()
    var ids = Object.keys(modules)
    compare(ids.length, 1)
    compare(ids[0], "acme.fileblade-weather/weather")
    var module = modules[ids[0]]
    compare(module.sourceDir, "/home/tester/.config/omarchy/plugins/acme.fileblade-weather")
    compare(String(module.entryUrl).indexOf("/acme.fileblade-weather/blades/Module.qml") >= 0, true)
    compare(module.providerId, "acme.fileblade-weather")
    compare(module.providerEntry, "Provider.qml")
    compare(module.compatible, true)
  }

  function test_a_disabled_companion_is_listed_as_disabled_not_loaded() {
    registry.catalogProviders = [{
      id: "acme.fileblade-weather",
      dir: "/home/tester/.config/omarchy/plugins/acme.fileblade-weather",
      manifest: companionManifest,
      enabled: false
    }]
    var modules = registry.manifestModules()
    compare(Object.keys(modules).length, 0)
    compare(Object.keys(registry.disabledModules).length, 1)
  }

  function test_the_registry_wins_when_the_shell_still_discloses_a_companion() {
    selfScopedRegistry.installedPlugins = ({
      "data-goblin.fileblade": { id: "data-goblin.fileblade" },
      "acme.fileblade-weather": Object.assign({ __sourceDir: "/shell/route" }, companionManifest)
    })
    selfScopedRegistry.enabledIds = ["data-goblin.fileblade", "acme.fileblade-weather"]
    registry.catalogProviders = [{
      id: "acme.fileblade-weather",
      dir: "/disk/route",
      manifest: companionManifest,
      enabled: true
    }]
    var modules = registry.manifestModules()
    var ids = Object.keys(modules)
    compare(ids.length, 1)
    compare(modules[ids[0]].sourceDir, "/shell/route")
    selfScopedRegistry.installedPlugins = ({ "data-goblin.fileblade": { id: "data-goblin.fileblade" } })
    selfScopedRegistry.enabledIds = ["data-goblin.fileblade"]
  }

  function test_the_catalog_accepts_only_complete_provider_rows() {
    compare(catalog.accepted(null), null)
    compare(catalog.accepted({ ok: false, providers: [] }), null)
    compare(catalog.accepted({ ok: true }), null)
    var rows = catalog.accepted({ ok: true, providers: [
      { id: "acme.one", dir: "/plugins/acme.one", manifest: companionManifest, enabled: true },
      { id: "", dir: "/plugins/nameless", manifest: companionManifest, enabled: true },
      { id: "acme.two", dir: "", manifest: companionManifest, enabled: true },
      { id: "acme.three", dir: "/plugins/acme.three", enabled: true }
    ]})
    compare(rows.length, 1)
    compare(rows[0].id, "acme.one")
    compare(rows[0].enabled, true)
  }

  function test_a_refresh_asks_the_backend_for_the_catalog() {
    fakeService.reply = { ok: true, activation: "known", providers: [
      { id: "acme.one", dir: "/plugins/acme.one", manifest: companionManifest, enabled: true }
    ]}
    catalog.refresh()
    compare(fakeService.lastRequest.name, "plugin-catalog")
    compare(catalog.activation, "known")
    compare(catalog.providers.length, 1)
    compare(catalog.error, "")
  }

  function test_an_unreadable_catalog_keeps_the_rows_but_drops_their_authority() {
    fakeService.reply = { ok: true, activation: "known", providers: [
      { id: "acme.one", dir: "/plugins/acme.one", manifest: companionManifest, enabled: true }
    ]}
    catalog.checkedAt = 0
    catalog.refresh()
    compare(catalog.providers[0].enabled, true)
    fakeService.reply = { ok: false, message: "no plugins directory" }
    catalog.checkedAt = 0
    catalog.refresh()
    compare(catalog.providers.length, 1)
    compare(catalog.providers[0].enabled, false)
    compare(catalog.activation, "unknown")
    compare(catalog.error, "no plugins directory")
  }

  function test_an_unknown_activation_never_reads_as_enabled() {
    fakeService.reply = { ok: true, activation: "unknown", providers: [
      { id: "acme.one", dir: "/plugins/acme.one", manifest: companionManifest, enabled: true }
    ]}
    catalog.checkedAt = 0
    catalog.refresh()
    compare(catalog.providers.length, 1)
    compare(catalog.providers[0].enabled, false)
  }

  function test_a_repeat_refresh_is_coalesced_but_a_watch_event_is_not() {
    fakeService.reply = { ok: true, activation: "known", providers: [] }
    catalog.checkedAt = 0
    compare(catalog.refresh(), true)
    compare(catalog.refresh(), false)
    catalog.watchPaths = ["/home/tester/.config/omarchy/shell.json", "/home/tester/.config/omarchy/plugins"]
    compare(catalog.watch(), true)
    compare(fakeService.watchPaths.length, 2)
    catalog.checkedAt = 0
    fakeService.emitWatchEvent()
    compare(fakeService.lastRequest.name, "plugin-catalog")
  }

  function test_the_provider_manager_refuses_an_unsafe_or_absent_provider_entry() {
    compare(providers.providerEntry(companionManifest), "Provider.qml")
    compare(providers.providerEntry({ extensions: { "data-goblin.fileblade/blade": [{ provider: "/etc/passwd" }] } }), "")
    compare(providers.providerEntry({ extensions: { "data-goblin.fileblade/blade": [{ provider: "../escape/Provider.qml" }] } }), "")
    compare(providers.providerEntry({ extensions: { "data-goblin.fileblade/blade": [{ provider: null }] } }), "")
    compare(providers.providerEntry({ extensions: { "data-goblin.fileblade/blade": [{ id: "weather" }] } }), "")
    compare(providers.providerEntry(null), "")
  }

  function test_a_disabled_or_rootless_provider_gets_no_runtime() {
    providers.providers = [
      { id: "acme.off", dir: "/plugins/acme.off", manifest: companionManifest, enabled: false },
      { id: "acme.rootless", dir: "", manifest: companionManifest, enabled: true }
    ]
    compare(Object.keys(providers.services).length, 0)
  }
}

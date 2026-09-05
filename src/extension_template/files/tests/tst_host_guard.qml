import QtQuick
import QtTest
import "../HostGuard.js" as HostGuard

TestCase {
  name: "HostGuard"

  readonly property string selfId: "{{PLUGIN_ID}}"
  readonly property string otherId: "data-goblin.fileblade-memory"
  readonly property var host: ({ id: "data-goblin.fileblade", name: "FileBlade" })
  readonly property var self: ({ id: selfId, name: "{{PLUGIN_NAME}}", extensions: { "data-goblin.fileblade/blade": [] } })
  readonly property var other: ({ id: otherId, name: "Memory", extensions: { "data-goblin.fileblade/helper": [] } })
  readonly property var unrelated: ({ id: "acme.weather", name: "Weather", extensions: { "acme.other/thing": [] } })

  function enabledExcept(disabled) {
    return function(id) { return disabled.indexOf(id) === -1 }
  }

  function installed(plugins) {
    var map = {}
    for (var i = 0; i < plugins.length; i++) map[plugins[i].id] = plugins[i]
    return map
  }

  function test_host_enabled_shows_nothing() {
    compare(HostGuard.plan(installed([host, self]), enabledExcept([]), selfId).show, false)
  }

  function test_host_missing_offers_install() {
    var plan = HostGuard.plan(installed([self, unrelated]), enabledExcept([]), selfId)
    compare(plan.show, true)
    compare(plan.action, "Install")
    compare(plan.command.slice(0, 2), ["sh", "-c"])
    verify(plan.command[2].indexOf("omarchy plugin add https://github.com/data-goblin/fileblade.git --enable --yes 2>&1") > 0)
    verify(plan.command[2].indexOf("exec omarchy restart shell") > 0)
    verify(plan.command[2].indexOf("omarchy-shell shell rescanPlugins") > 0)
    compare(plan.names, ["{{PLUGIN_NAME}}"])
  }

  function test_only_the_first_extension_shows_and_lists_all() {
    var plugins = installed([self, other])
    var first = [selfId, otherId].sort()[0]
    var second = first === selfId ? otherId : selfId
    compare(HostGuard.plan(plugins, enabledExcept([]), first).show, true)
    compare(HostGuard.plan(plugins, enabledExcept([]), second).show, false)
    compare(HostGuard.plan(plugins, enabledExcept([]), first).names.slice().sort(), ["Memory", "{{PLUGIN_NAME}}"].sort())
    compare(HostGuard.plan(plugins, enabledExcept([first]), second).show, true)
  }

  function test_host_installed_but_disabled_offers_enable() {
    var plan = HostGuard.plan(installed([host, self]), enabledExcept(["data-goblin.fileblade"]), selfId)
    compare(plan.show, true)
    compare(plan.action, "Enable")
    verify(plan.command[2].indexOf("omarchy plugin enable data-goblin.fileblade 2>&1") > 0)
  }

  function test_no_registry_data_shows_nothing() {
    compare(HostGuard.plan(null, enabledExcept([]), selfId).show, false)
  }
}

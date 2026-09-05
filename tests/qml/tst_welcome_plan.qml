import QtQuick
import QtTest
import "../../modules/welcome/WelcomePlan.js" as WelcomePlan

TestCase {
  name: "WelcomePlan"

  function test_copy_is_the_agreed_text() {
    compare(WelcomePlan.HEADING, "Install agent extensions?")
    verify(WelcomePlan.BODY.indexOf("agent memory files, skills, MCPs, and hooks") > 0)
    verify(WelcomePlan.BODY.indexOf("ask your agent to put an existing Omarchy plugin here") > 0)
    compare(WelcomePlan.DISMISS, "Close and don't show this again")
  }

  function test_four_extensions_install_through_omarchy_plugin_add() {
    compare(WelcomePlan.EXTENSIONS.map(function(item) { return item.name }), ["Memory", "Skills", "MCP", "Hooks"])
    for (var i = 0; i < WelcomePlan.EXTENSIONS.length; i++) {
      var command = WelcomePlan.installCommand(WelcomePlan.EXTENSIONS[i])
      compare(command[0], "omarchy-plugin-add")
      verify(command[1].indexOf("https://github.com/data-goblin/fileblade-") === 0)
      compare(command.slice(2), ["--yes", "--enable"])
    }
  }

  function test_only_missing_extensions_are_queued() {
    var queued = WelcomePlan.missing(function(id) { return id === "data-goblin.fileblade-skills/skills" })
    compare(queued.map(function(item) { return item.name }), ["Memory", "MCP", "Hooks"])
    compare(WelcomePlan.missing(function() { return true }).length, 0)
  }

  function test_state_pending_only_while_unset() {
    verify(WelcomePlan.pending(""))
    verify(WelcomePlan.pending(undefined))
    verify(!WelcomePlan.pending("dismissed"))
    verify(!WelcomePlan.pending("installed"))
    compare(WelcomePlan.normalizeState("bogus"), "")
    compare(WelcomePlan.welcomeSlot().modules[0].module, "welcome")
    compare(WelcomePlan.welcomeSlot().modules[1].module, "notes")
  }
}

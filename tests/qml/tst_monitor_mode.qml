import QtQuick
import QtTest
import "../../lib/MonitorMode.js" as MonitorMode

TestCase {
  name: "MonitorMode"

  function test_normalize_accepts_active_all_locked_and_maps_primary_to_locked() {
    compare(MonitorMode.normalize("active"), "active")
    compare(MonitorMode.normalize("All"), "all")
    compare(MonitorMode.normalize("locked"), "locked")
    compare(MonitorMode.normalize("primary"), "locked")
    compare(MonitorMode.normalize(""), "active")
    compare(MonitorMode.normalize(undefined), "active")
    compare(MonitorMode.normalize("bogus"), "active")
  }

  function test_all_mode_mirrors_on_every_named_screen() {
    verify(MonitorMode.eligible("all", "DP-1", { lock: "", openedOn: "" }))
    verify(MonitorMode.eligible("all", "HDMI-A-1", { lock: "", openedOn: "DP-1" }))
    verify(!MonitorMode.eligible("all", "", {}))
  }

  function test_locked_mode_only_shows_on_the_lock() {
    verify(MonitorMode.eligible("locked", "HDMI-A-1", { lock: "HDMI-A-1", openedOn: "DP-1" }))
    verify(!MonitorMode.eligible("locked", "DP-1", { lock: "HDMI-A-1", openedOn: "DP-1" }))
    verify(!MonitorMode.eligible("locked", "HDMI-A-1", { lock: "", openedOn: "HDMI-A-1" }))
  }

  function test_active_mode_shows_only_on_the_invocation_screen() {
    verify(MonitorMode.eligible("active", "DP-1", { openedOn: "DP-1" }))
    verify(!MonitorMode.eligible("active", "HDMI-A-1", { openedOn: "DP-1" }))
    verify(!MonitorMode.eligible("active", "DP-1", { openedOn: "" }))
  }

  function test_invocation_name_captures_focus_lock_or_nothing() {
    compare(MonitorMode.invocationName("active", { focused: "HDMI-A-1", lock: "DP-1" }), "HDMI-A-1")
    compare(MonitorMode.invocationName("active", { focused: "", lock: "DP-1" }), "")
    compare(MonitorMode.invocationName("locked", { focused: "HDMI-A-1", lock: "DP-1" }), "DP-1")
    compare(MonitorMode.invocationName("all", { focused: "HDMI-A-1", lock: "DP-1" }), "")
  }

  function test_preferred_name_prefers_the_open_blade_then_mode_rules() {
    compare(MonitorMode.preferredName("active", { openedOn: "DP-1", focused: "HDMI-A-1", primary: "DP-1" }), "DP-1")
    compare(MonitorMode.preferredName("active", { openedOn: "", focused: "HDMI-A-1", primary: "DP-1" }), "HDMI-A-1")
    compare(MonitorMode.preferredName("active", { openedOn: "", focused: "", primary: "DP-1" }), "")
    compare(MonitorMode.preferredName("locked", { openedOn: "", focused: "HDMI-A-1", lock: "DP-2", primary: "DP-1" }), "DP-2")
    compare(MonitorMode.preferredName("all", { openedOn: "", focused: "HDMI-A-1", primary: "DP-1" }), "DP-1")
  }

  function test_settings_choices_round_trip_through_keys() {
    var rows = MonitorMode.choices(["DP-1", "HDMI-A-1"])
    compare(rows.map(function(row) { return row.key }), ["active", "all", "lock:DP-1", "lock:HDMI-A-1"])
    compare(rows[3].label, "Lock to HDMI-A-1")
    compare(MonitorMode.choiceKey("locked", "DP-1"), "lock:DP-1")
    compare(MonitorMode.choiceKey("active", ""), "active")
    compare(MonitorMode.parseChoice("lock:HDMI-A-1"), { mode: "locked", lock: "HDMI-A-1" })
    compare(MonitorMode.parseChoice("all"), { mode: "all", lock: "" })
  }
}

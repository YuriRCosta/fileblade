import QtQuick
import QtTest
import "../../lib/TabIdentity.js" as TabIdentity

TestCase {
  name: "TabIdentity"

  readonly property var tabs: [{ module: "notes", state: { id: 1 } }, { module: "notes", state: { id: 2 } }, { module: "files", state: {} }]

  function test_capture_records_index_slot_and_a_snapshot_of_every_tab() {
    var captured = TabIdentity.capture(tabs, 1, "notes-slot")
    compare(captured.index, 1)
    compare(captured.slotId, "notes-slot")
    verify(captured.snapshot.indexOf("\"id\":2") >= 0)
    compare(TabIdentity.capture(tabs, 3, "notes-slot"), null)
    compare(TabIdentity.capture(null, 0, "notes-slot"), null)
  }

  function test_any_real_tab_list_change_invalidates_the_prompt() {
    var captured = TabIdentity.capture(tabs, 1, "notes-slot")
    verify(TabIdentity.matches(tabs, "notes-slot", captured))
    verify(!TabIdentity.matches([tabs[1], tabs[2]], "notes-slot", captured), "earlier tab removed")
    verify(!TabIdentity.matches([tabs[1], tabs[0], tabs[2]], "notes-slot", captured), "same-module tabs swapped")
    verify(!TabIdentity.matches(tabs.concat([{ module: "files", state: {} }]), "notes-slot", captured), "tab added")
    verify(!TabIdentity.matches(tabs, "other-slot", captured), "slot replaced")
    verify(!TabIdentity.matches(tabs, "notes-slot", null))
  }

  function test_unchanged_tabs_keep_the_prompt_valid() {
    var captured = TabIdentity.capture(tabs, 2, "files-slot")
    var same = [{ module: "notes", state: { id: 1 } }, { module: "notes", state: { id: 2 } }, { module: "files", state: {} }]
    verify(TabIdentity.matches(same, "files-slot", captured))
  }
}

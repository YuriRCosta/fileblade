import QtQuick
import QtTest
import "../../lib/TabIdentity.js" as TabIdentity

TestCase {
  name: "TabIdentity"

  readonly property var tabs: [{ module: "a" }, { module: "b" }, { module: "c" }]

  function test_capture_records_index_module_and_count() {
    compare(TabIdentity.capture(tabs, 1), { index: 1, module: "b", count: 3 })
    compare(TabIdentity.capture(tabs, 3), null)
    compare(TabIdentity.capture(tabs, -1), null)
    compare(TabIdentity.capture(null, 0), null)
  }

  function test_removing_an_earlier_tab_invalidates_the_captured_target() {
    var captured = TabIdentity.capture(tabs, 1)
    verify(TabIdentity.matches(tabs, captured))
    verify(!TabIdentity.matches([{ module: "b" }, { module: "c" }], captured), "index 1 is now c")
    verify(!TabIdentity.matches([{ module: "a" }, { module: "c" }, { module: "b" }], captured), "same count, different module")
    verify(!TabIdentity.matches([{ module: "a" }, { module: "b" }, { module: "c" }, { module: "d" }], captured), "count changed")
    verify(!TabIdentity.matches(tabs, null))
  }
}

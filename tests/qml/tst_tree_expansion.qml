import QtQuick
import QtTest
import "../../controllers"

TestCase {
  name: "TreeExpansion"
  property var pump: null
  property var pending: []
  property var opened: []
  Component { id: component; TreeExpansion {} }

  function init() {
    pending = []; opened = []
    pump = createTemporaryObject(component, this, {
      nextEntry: function() { return pending.length ? pending[0] : null },
      expandEntry: function(entry) { opened.push(entry.path); pending.shift() }
    })
    verify(pump !== null)
  }

  function test_idle_does_no_work_and_async_revisions_continue_the_same_operation() {
    pending = [{ path: "parent", depth: 0 }]
    wait(30); compare(opened.length, 0)
    pump.loading = true; pump.start()
    tryCompare(pump, "steps", 1)
    verify(pump.running)
    pending.push({ path: "child", depth: 1 })
    pump.revision = 1
    tryCompare(pump, "steps", 2)
    pump.loading = false
    tryCompare(pump, "running", false)
    compare(opened, ["parent", "child"])
  }

  function test_cancel_and_hide_stop_queued_work_even_after_a_late_result() {
    pending = [{ path: "parent", depth: 0 }]
    pump.start(); pump.stop(true)
    pump.revision = 1
    wait(30); compare(opened.length, 0)
    pump.start(); pump.active = false
    pump.revision = 2
    wait(30); compare(opened.length, 0)
    pump.active = true
    wait(30); verify(!pump.running)
    pump.start()
    tryCompare(pump, "running", false)
    compare(opened, ["parent"])
  }

  function test_budgets_data() {
    return [
      { tag: "steps", property: "maximumSteps", value: 1, depth: 1, expected: 1 },
      { tag: "rows", property: "maximumRows", value: 1, depth: 1, expected: 0 },
      { tag: "depth", property: "maximumDepth", value: 0, depth: 1, expected: 1 }
    ]
  }

  function test_budgets(data) {
    pump[data.property] = data.value
    pump.rowCount = 1
    pending = [{ path: "parent", depth: 0 }, { path: "child", depth: data.depth }]
    pump.start()
    tryCompare(pump, "running", false)
    compare(opened.length, data.expected)
    verify(pump.error.indexOf("select a smaller folder") >= 0)
    pump.stop(true); compare(pump.error, "")
  }
}

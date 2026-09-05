import QtQuick
import QtTest
import "../../lib/WheelGeometry.js" as Wheel

TestCase {
  name: "WheelGeometryRegression"

  readonly property real quarter: Math.PI / 2

  function wheel(count, outerCount, parentIndex) {
    return {
      x: 500,
      y: 500,
      hubRadius: 26,
      outerRadius: 93,
      childOuterRadius: 144,
      count: count,
      outerCount: outerCount,
      parentAngle: parentIndex >= 0 ? Wheel.wedgeAngle(count, parentIndex) : 0
    }
  }

  function at(model, angle, radius) {
    return Wheel.pointAt(model, model.x + Math.cos(angle) * radius, model.y + Math.sin(angle) * radius)
  }

  function test_first_wedge_sits_at_the_top() {
    compare(Wheel.wedgeAngle(6, 0), -quarter)
    compare(Wheel.wedgeAtAngle(6, -quarter), 0)
    compare(Wheel.wedgeAtAngle(6, -quarter + 2 * Math.PI / 6), 1)
    compare(Wheel.wedgeAtAngle(6, -quarter - 0.01), 0)
    compare(Wheel.wedgeAtAngle(6, -quarter - 2 * Math.PI / 6), 5)
  }

  function test_children_fan_out_around_the_parent() {
    var parent = Wheel.wedgeAngle(6, 2)
    compare(Wheel.childStep(3), Math.PI / 4)
    fuzzyCompare(Wheel.childAngle(parent, 3, 1), parent, 1e-9)
    fuzzyCompare(Wheel.childAngle(parent, 3, 0), parent - Math.PI / 4, 1e-9)
    fuzzyCompare(Wheel.childAngle(parent, 3, 2), parent + Math.PI / 4, 1e-9)
    compare(Wheel.childAtAngle(parent, 3, parent), 1)
    compare(Wheel.childAtAngle(parent, 3, parent - Math.PI / 4), 0)
    compare(Wheel.childAtAngle(parent, 3, parent + Math.PI / 4 + 0.1), 2)
    compare(Wheel.childAtAngle(parent, 3, parent + Math.PI / 2), -1)
    compare(Wheel.childAtAngle(parent, 3, parent - 2 * Math.PI), 1)
  }

  function test_eight_children_close_the_ring() {
    compare(Wheel.childStep(8), Math.PI / 4)
    compare(Wheel.childAtAngle(-quarter, 8, -quarter + Math.PI - 0.01), 7)
    compare(Wheel.childAtAngle(-quarter, 8, -quarter - Math.PI + 0.01), 0)
    verify(Wheel.childStep(12) < Math.PI / 4)
  }

  function test_hub_and_far_away_miss() {
    var model = wheel(6, 0, -1)
    compare(at(model, 0, 10).ring, "none")
    compare(at(model, 0, 93 + 29).ring, "none")
    compare(at(model, 0, 93 + 27).ring, "inner")
  }

  function test_outer_band_belongs_to_the_open_parent() {
    var model = wheel(6, 3, 0)
    var top = -quarter
    compare(at(model, top, 60).ring, "inner")
    compare(at(model, top, 60).index, 0)
    compare(at(model, top, 120).ring, "outer")
    compare(at(model, top, 120).index, 1)
    compare(at(model, top, 96).ring, "outer")
    compare(at(model, top + Math.PI / 4, 120).index, 2)
    compare(at(model, top, 144 + 27).ring, "outer")
    compare(at(model, top, 144 + 29).ring, "none")
  }

  function test_outside_the_arc_the_band_falls_back_to_the_inner_wedge() {
    var model = wheel(6, 3, 0)
    var hit = at(model, -quarter + Math.PI, 120)
    compare(hit.ring, "inner")
    compare(hit.index, 3)
  }
}

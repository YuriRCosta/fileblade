import QtQuick
import QtTest
import "../../lib/Definitions.js" as Definitions
import "../../lib/SettingsForm.js" as Form

TestCase {
  name: "SettingsFormRegression"

  function row(key, type, extra) {
    var result = { key: key, type: type }
    if (extra) for (var name in extra) result[name] = extra[name]
    return result
  }

  function schema(rows) {
    return Definitions.settingsSpec({ schema: rows }).schema
  }

  function only(rowSpec) {
    return schema([rowSpec])[0]
  }

  function hostile() {
    return "a" + String.fromCharCode(0) + "bcd" + String.fromCharCode(0x2066) + "e" + String.fromCharCode(0x1f)
  }

  function test_row_lookup_ignores_prototype_names_and_garbage() {
    var rows = schema([row("format", "string"), row("refresh", "integer")])
    compare(Form.row(rows, "refresh").key, "refresh")
    compare(Form.row(rows, "missing"), null)
    compare(Form.row(rows, "constructor"), null)
    compare(Form.row(rows, "toString"), null)
    compare(Form.row(rows, undefined), null)
    compare(Form.row(undefined, "format"), null)
    compare(Form.row([null, 3, "x"], "format"), null)
  }

  function test_integer_coerce_floors_and_clamps() {
    var refresh = only(row("refresh", "integer", { min: 1, max: 3600, step: 5, defaultValue: 60 }))
    compare(Form.coerce(refresh, 4.9), 4)
    compare(Form.coerce(refresh, "12"), 12)
    compare(Form.coerce(refresh, 99999), 3600)
    compare(Form.coerce(refresh, -7), 1)
    compare(Form.coerce(refresh, "abc"), undefined)
    compare(Form.coerce(refresh, ""), undefined)
    compare(Form.coerce(refresh, true), undefined)
    compare(Form.coerce(refresh, { nested: 1 }), undefined)
    compare(Form.coerce(refresh, "1e400"), undefined)
    compare(Form.effective(refresh, "abc"), 60)
    compare(Form.effective(refresh, undefined), 60)
    compare(Form.effective(refresh, null), 60)
    compare(Form.effective(refresh, 2), 2)
  }

  function test_number_coerce_clamps_and_keeps_fractions() {
    var scale = only(row("scale", "number", { min: 0.5, max: 2, step: 0.1, defaultValue: 1 }))
    compare(Form.coerce(scale, 1.25), 1.25)
    compare(Form.coerce(scale, "0.75"), 0.75)
    compare(Form.coerce(scale, 9), 2)
    compare(Form.coerce(scale, -1), 0.5)
    compare(Form.coerce(scale, "nope"), undefined)
    compare(Form.effective(scale, "nope"), 1)
  }

  function test_enum_coerce_falls_back_to_the_default() {
    var style = only(row("style", "enum", { options: ["compact", { value: "wide", label: "Wide" }], defaultValue: "wide" }))
    compare(Form.coerce(style, "compact"), "compact")
    compare(Form.coerce(style, "wide"), "wide")
    compare(Form.coerce(style, "huge"), undefined)
    compare(Form.coerce(style, { value: "wide" }), undefined)
    compare(Form.effective(style, "huge"), "wide")
    compare(Form.optionLabel(style, "wide"), "Wide")
    compare(Form.optionLabel(style, "compact"), "compact")
    compare(Form.optionLabel(style, "huge"), "Wide")
    compare(Form.choiceOptions(style), [{ key: "compact", label: "compact" }, { key: "wide", label: "Wide" }])
    compare(Form.popupRows(style, "compact"), [{ key: "compact", label: "compact", checked: true }, { key: "wide", label: "Wide", checked: false }])
    compare(Form.popupRows(style, "huge")[1].checked, true)
  }

  function test_inline_choice_switches_to_a_popup_above_four_options() {
    var four = only(row("a", "enum", { options: ["1", "2", "3", "4"] }))
    var five = only(row("b", "enum", { options: ["1", "2", "3", "4", "5"] }))
    verify(Form.inlineChoice(four))
    verify(!Form.inlineChoice(five))
    verify(!Form.inlineChoice(only(row("c", "string"))))
    verify(!Form.inlineChoice(null))
  }

  function test_string_coerce_caps_length_and_strips_controls() {
    var format = only(row("format", "string", { maxLength: 8, defaultValue: "HH:mm" }))
    compare(Form.coerce(format, "abcdefghijk"), "abcdefgh")
    compare(Form.coerce(format, hostile()), "abcde")
    compare(Form.coerce(format, 12), "12")
    compare(Form.coerce(format, ""), "")
    compare(Form.coerce(format, { x: 1 }), undefined)
    compare(Form.coerce(format, ["x"]), undefined)
    compare(Form.effective(format, { x: 1 }), "HH:mm")
    var long = new Array(6000).join("y")
    var wide = only(row("wide", "string", { maxLength: 999999 }))
    compare(Form.coerce(wide, long).length, 4096)
  }

  function test_boolean_coerce_accepts_strings() {
    var flag = only(row("flag", "boolean", { defaultValue: true }))
    compare(Form.coerce(flag, "false"), false)
    compare(Form.coerce(flag, "true"), true)
    compare(Form.coerce(flag, 0), false)
    compare(Form.coerce(flag, 1), true)
    compare(Form.coerce(flag, "no"), true)
    compare(Form.coerce(flag, []), undefined)
    compare(Form.effective(flag, undefined), true)
  }

  function test_path_rows_pass_text_through_bounded() {
    var folder = only(row("folder", "path"))
    compare(Form.coerce(folder, "/srv/projects"), "/srv/projects")
    compare(Form.coerce(folder, "~/" + hostile()), "~/abcde")
    compare(Form.coerce(folder, new Array(300).join("p")).length, 256)
    compare(Form.effective(folder, undefined), "")
  }

  function test_coerce_without_a_row_is_undefined() {
    compare(Form.coerce(null, "x"), undefined)
    compare(Form.coerce(undefined, 1), undefined)
    compare(Form.coerce("string", 1), undefined)
  }

  function test_step_rounds_to_the_step_precision_and_clamps() {
    var scale = only(row("scale", "number", { min: 0.5, max: 2, step: 0.1, defaultValue: 1 }))
    compare(Form.stepValue(scale, 1, 1), 1.1)
    compare(Form.stepValue(scale, 1.1, 1), 1.2)
    compare(Form.stepValue(scale, 0.6, -1), 0.5)
    compare(Form.stepValue(scale, 0.5, -1), 0.5)
    compare(Form.stepValue(scale, 1.95, 1), 2)
    compare(Form.stepValue(scale, "garbage", 1), 1.1)
    var refresh = only(row("refresh", "integer", { min: 1, max: 3600, step: 5, defaultValue: 60 }))
    compare(Form.stepValue(refresh, 60, 1), 65)
    compare(Form.stepValue(refresh, 3, -1), 1)
    compare(Form.stepValue(refresh, 3598, 1), 3600)
    var tiny = only(row("tiny", "number", { min: 0, max: 1, step: 0.001 }))
    compare(Form.stepValue(tiny, 0.1, 1), 0.101)
    compare(Form.stepValue(tiny, 0.3, 1), 0.301)
  }

  function test_format_value_prints_integers_whole_and_numbers_trimmed() {
    var refresh = only(row("refresh", "integer", { min: 1, max: 3600 }))
    var scale = only(row("scale", "number", { min: 0.5, max: 2, step: 0.1 }))
    compare(Form.formatValue(refresh, 60), "60")
    compare(Form.formatValue(refresh, 4.7), "4")
    compare(Form.formatValue(refresh, "abc"), "1")
    compare(Form.formatValue(scale, 1), "1")
    compare(Form.formatValue(scale, 1.2000000000000002), "1.2")
    compare(Form.formatValue(scale, 1.25), "1.3")
    compare(Form.formatValue(scale, 0.15), "0.5")
    compare(Form.formatValue(only(row("s", "string")), "text"), "text")
    compare(Form.formatValue(only(row("s", "string")), undefined), "")
    compare(Form.decimals(0.1), 1)
    compare(Form.decimals(1), 0)
    compare(Form.decimals(0.001), 3)
    compare(Form.decimals(1e-12), 12)
    compare(Form.decimals(2.5e-7), 8)
    compare(Form.decimals(1e-300), 12)
  }

  function test_range_text_shows_only_declared_bounds() {
    compare(Form.rangeText(only(row("a", "integer", { min: 1, max: 3600 }))), "1 to 3600")
    compare(Form.rangeText(only(row("b", "integer", { min: 1 }))), "from 1")
    compare(Form.rangeText(only(row("c", "number", { max: 2.5 }))), "up to 2.5")
    compare(Form.rangeText(only(row("d", "number"))), "")
    compare(Form.rangeText(only(row("e", "string"))), "")
    compare(Form.rangeText(null), "")
  }

  function test_row_model_carries_effective_values_and_detail() {
    var rows = schema([
      row("format", "string", { label: "Format", description: "Qt date format", placeholder: "HH:mm", defaultValue: "HH:mm" }),
      row("showSeconds", "boolean"),
      row("refresh", "integer", { min: 1, max: 3600, step: 5, defaultValue: 60 }),
      row("style", "enum", { options: ["compact", "wide"] })
    ])
    var model = Form.rowModel(rows, { format: "yyyy", refresh: "9999", style: "nope", showSeconds: "true" })
    compare(model.length, 4)
    compare(model[0].value, "yyyy")
    compare(model[0].label, "Format")
    compare(model[0].description, "Qt date format")
    compare(model[0].placeholder, "HH:mm")
    compare(model[0].detail, "")
    compare(model[1].value, true)
    compare(model[2].value, 3600)
    compare(model[2].detail, "1 to 3600")
    compare(model[2].step, 5)
    compare(model[3].value, "compact")
    compare(model[3].options.length, 2)
    var defaults = Form.rowModel(rows, undefined)
    compare(defaults[0].value, "HH:mm")
    compare(defaults[1].value, false)
    compare(defaults[2].value, 60)
    compare(Form.rowModel(rows, "junk")[2].value, 60)
    compare(Form.rowModel(undefined, {}).length, 0)
    compare(Form.rowModel([null, "x", {}, { key: 3 }], {}).length, 0)
  }

  function test_row_model_carries_the_group_name() {
    var rows = Form.rowModel([
      { key: "a", type: "boolean", label: "A", group: "Look" },
      { key: "b", type: "boolean", label: "B" }
    ], {})
    compare(rows[0].group, "Look")
    compare(rows[1].group, "")
  }

  function test_row_model_reads_own_properties_only() {
    var rows = schema([row("constructor_", "string", { defaultValue: "d" }), row("hasOwnProperty_", "string")])
    var values = Object.create({ constructor_: "inherited" })
    compare(Form.rowModel(rows, values)[0].value, "d")
    compare(Form.storedValue({ hasOwnProperty_: "own" }, "hasOwnProperty_"), "own")
    compare(Form.storedValue(null, "x"), undefined)
    compare(Form.storedValue(7, "x"), undefined)
  }

  function test_row_model_is_capped_at_the_schema_limit() {
    var rows = []
    for (var i = 0; i < 40; i++) rows.push({ key: "k" + i, type: "boolean", defaultValue: false })
    compare(Form.rowModel(rows, {}).length, 32)
  }
}

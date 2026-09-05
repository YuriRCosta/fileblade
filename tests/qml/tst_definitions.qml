import QtQuick
import QtTest
import "../../lib/Definitions.js" as Definitions
import "../fixtures/module_dir_names.js" as Fixture

TestCase {
  name: "DefinitionsRegression"

  function row(key, type, extra) {
    var result = { key: key, type: type }
    if (extra) for (var name in extra) result[name] = extra[name]
    return result
  }

  function spec(schema, defaults) {
    return Definitions.settingsSpec({ schema: schema, defaults: defaults || ({}) })
  }

  function keys(result) {
    return result.schema.map(function(entry) { return entry.key })
  }

  function test_bounded_text_strips_controls_and_slices() {
    compare(Definitions.boundedText("a\u0000b\u001fc\u007fd", "", 10), "abcd")
    compare(Definitions.boundedText(undefined, "fallback", 4), "fall")
    compare(Definitions.boundedText(null, "x", 4), "x")
    compare(Definitions.boundedText(12, "", 4), "12")
    compare(Definitions.boundedText("ab\u0085c\u202ed\u2066e", "", 20), "abcde")
  }

  function test_safe_id_accepts_the_bare_id_charset_only() {
    compare(Definitions.safeId("notes", 128), "notes")
    compare(Definitions.safeId("  clock ", 128), "clock")
    compare(Definitions.safeId("a.b_c-d", 128), "a.b_c-d")
    compare(Definitions.safeId("a/b", 128), "")
    compare(Definitions.safeId(".hidden", 128), "")
    compare(Definitions.safeId("-x", 128), "")
    compare(Definitions.safeId("", 128), "")
    compare(Definitions.safeId(undefined, 128), "")
    compare(Definitions.safeId("abcdef", 5), "")
    compare(Definitions.safeId("abcde", 5), "abcde")
  }

  function test_module_id_pattern_matches_one_optional_provider_segment() {
    verify(Definitions.MODULE_ID_PATTERN.test("notes"))
    verify(Definitions.MODULE_ID_PATTERN.test("data-goblin.blade-example/clock"))
    verify(!Definitions.MODULE_ID_PATTERN.test("a/b/c"))
    verify(!Definitions.MODULE_ID_PATTERN.test("/x"))
    verify(!Definitions.MODULE_ID_PATTERN.test("x/"))
    verify(!Definitions.MODULE_ID_PATTERN.test("a+b"))
  }

  function test_dir_name_matches_the_shared_fixture() {
    verify(Fixture.NAMES.length >= 10)
    for (var i = 0; i < Fixture.NAMES.length; i++)
      compare(Definitions.dirName(Fixture.NAMES[i].id), Fixture.NAMES[i].name, Fixture.NAMES[i].id)
    compare(Definitions.dirName(undefined), "")
    compare(Definitions.dirName(null), "")
  }

  function test_dir_name_keeps_object_prototype_member_names_usable_as_table_keys() {
    var inherited = ["constructor", "toString", "valueOf", "hasOwnProperty", "isPrototypeOf", "propertyIsEnumerable", "toLocaleString"]
    var table = Object.create(null)
    for (var i = 0; i < inherited.length; i++) {
      var name = Definitions.dirName(inherited[i])
      compare(name, inherited[i])
      compare(table[name], undefined)
      table[name] = { ready: false, result: null, pending: [] }
      compare(table[name].ready, false)
      compare(table[name].pending.length, 0)
      table[name].pending.push(function() {})
      compare(table[name].pending.length, 1)
      delete table[name]
      compare(table[name], undefined)
    }
    compare(Object.keys(table).length, 0)
  }

  function test_settings_rows_keep_a_trimmed_bounded_group() {
    var rows = spec([
      { key: "a", type: "boolean", group: "  Look " },
      { key: "b", type: "boolean" },
      { key: "c", type: "boolean", group: null },
      { key: "d", type: "boolean", group: "x".repeat(80) }
    ]).schema
    compare(rows[0].group, "Look")
    compare(rows[1].group, "")
    compare(rows[2].group, "")
    compare(rows[3].group.length, 64)
  }

  function test_category_defaults_per_source() {
    compare(Definitions.category(undefined, "builtin"), "Module")
    compare(Definitions.category(undefined, "user"), "Module")
    compare(Definitions.category(undefined, "plugin:data-goblin.blade-example"), "Plugin")
    compare(Definitions.category("", "plugin:x"), "Plugin")
    compare(Definitions.category(null, ""), "Module")
    compare(Definitions.category(7, "plugin:x"), "Plugin")
  }

  function test_category_is_bounded_to_the_pattern() {
    compare(Definitions.category("Agents", "builtin"), "Agents")
    compare(Definitions.category("  Agent tools  ", "builtin"), "Agent tools")
    compare(Definitions.category("Two 2", "plugin:x"), "Two 2")
    compare(Definitions.category("a\u0000b", "builtin"), "ab")
    compare(Definitions.category("9lives", "builtin"), "Module")
    compare(Definitions.category("Agents/tools", "builtin"), "Module")
    compare(Definitions.category("<b>x</b>", "plugin:x"), "Plugin")
    compare(Definitions.category("abcdefghijklmnopqrstuvwxyzabcdef", "builtin"), "abcdefghijklmnopqrstuvwxyzabcdef")
    compare(Definitions.category("abcdefghijklmnopqrstuvwxyzabcdefg", "builtin"), "Module")
  }

  function test_category_order_puts_module_first_and_plugin_last() {
    var names = ["Plugin", "Widgets", "Module", "Agents"]
    names.sort(Definitions.categoryOrder)
    compare(names, ["Module", "Agents", "Widgets", "Plugin"])
    compare(Definitions.categoryOrder("Module", "Module"), 0)
    verify(Definitions.categoryOrder("Module", "Agents") < 0)
    verify(Definitions.categoryOrder("Plugin", "Zzz") > 0)
  }

  function test_settings_spec_tolerates_garbage() {
    compare(Definitions.settingsSpec(undefined).schema.length, 0)
    compare(Definitions.settingsSpec(null).schema.length, 0)
    compare(Definitions.settingsSpec("x").schema.length, 0)
    compare(Definitions.settingsSpec([]).schema.length, 0)
    compare(Definitions.settingsSpec({ schema: "no", defaults: 3 }).schema.length, 0)
    var result = spec([null, 4, "row", [], row("ok", "boolean")], [])
    compare(keys(result), ["ok"])
    compare(Object.keys(result.defaults), ["ok"])
  }

  function test_settings_spec_caps_rows_and_drops_duplicates_reserved_and_unknown() {
    var rows = []
    for (var i = 0; i < 33; i++) rows.push(row("k" + i, "boolean"))
    compare(spec(rows).schema.length, 32)
    var result = spec([
      row("format", "string"),
      row("format", "boolean"),
      row("id", "string"),
      row("mystery", "colour"),
      row("bad key", "string"),
      row("1st", "string"),
      row("_ok", "string"),
      row("abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklm", "string"),
      row("noOptions", "enum"),
      row("emptyOptions", "enum", { options: [] }),
      row("junkOptions", "enum", { options: [null, {}, { label: "x" }, ""] })
    ])
    compare(keys(result), ["format", "_ok"])
    compare(result.schema[0].type, "string")
  }

  function test_settings_spec_labels_and_descriptions_are_bounded() {
    var long = new Array(200).join("x")
    var result = spec([row("a", "boolean", { label: long, description: long }), row("b", "boolean", { label: "  " })])
    compare(result.schema[0].label.length, 64)
    compare(result.schema[0].description.length, 160)
    compare(result.schema[1].label, "b")
  }

  function test_default_precedence_is_row_then_defaults_then_type_zero() {
    var result = spec([
      row("a", "integer", { defaultValue: 4 }),
      row("b", "integer"),
      row("c", "integer"),
      row("d", "boolean"),
      row("e", "enum", { options: ["x", "y"] }),
      row("f", "string"),
      row("g", "path"),
      row("h", "number", { min: 0.5, max: 2 }),
      row("i", "integer", { min: 3, max: 9 })
    ], { a: 9, b: 7, c: "not a number", d: "true", e: "y", f: 12, g: "/tmp" })
    compare(result.defaults, { a: 4, b: 7, c: 0, d: true, e: "y", f: "12", g: "/tmp", h: 0.5, i: 3 })
    compare(result.schema[0].defaultValue, 4)
    compare(result.schema[2].defaultValue, 0)
  }

  function test_integer_rows_floor_and_clamp() {
    var result = spec([
      row("a", "integer", { min: 1, max: 10, step: 0.4, defaultValue: 4.9 }),
      row("b", "integer", { min: 1, max: 10, defaultValue: 99 }),
      row("c", "integer", { min: 1, max: 10, defaultValue: -3 }),
      row("d", "integer", { min: 10, max: 1, step: "2" }),
      row("e", "integer", { min: -1e12, max: 1e12 }),
      row("f", "integer", { min: 1.5, max: 9.5 })
    ])
    compare(result.schema[0].defaultValue, 4)
    compare(result.schema[0].step, 1)
    compare(result.schema[1].defaultValue, 10)
    compare(result.schema[2].defaultValue, 1)
    compare(result.schema[3].min, -1e9)
    compare(result.schema[3].max, 1e9)
    compare(result.schema[3].step, 2)
    compare(result.schema[4].min, -1e9)
    compare(result.schema[4].max, 1e9)
    compare(result.schema[5].min, 2)
    compare(result.schema[5].max, 9)
  }

  function test_number_rows_clamp_and_keep_fractions() {
    var result = spec([
      row("a", "number", { min: 0.5, max: 2, step: 0.1, defaultValue: 1.25 }),
      row("b", "number", { min: 0.5, max: 2, defaultValue: 7 }),
      row("c", "number", { step: -1, defaultValue: "1e400" }),
      row("d", "number", { min: "abc", max: true, step: 0 })
    ])
    compare(result.schema[0].defaultValue, 1.25)
    compare(result.schema[1].defaultValue, 2)
    compare(result.schema[2].step, 0.1)
    compare(result.schema[2].defaultValue, 0)
    compare(result.schema[3].min, -1e9)
    compare(result.schema[3].max, 1e9)
    compare(result.schema[3].step, 0.1)
  }

  function test_enum_rows_normalize_object_options_and_fall_back() {
    var many = []
    for (var i = 0; i < 40; i++) many.push("o" + i)
    var result = spec([
      row("style", "enum", { options: ["compact", { value: "wide", label: "Wide" }, { value: "wide" }, 3, { value: 3 }], defaultValue: "wide" }),
      row("fallback", "enum", { options: ["x", "y"], defaultValue: "z" }),
      row("many", "enum", { options: many })
    ])
    compare(result.schema[0].options, [{ value: "compact", label: "compact" }, { value: "wide", label: "Wide" }, { value: "3", label: "3" }])
    compare(result.schema[0].defaultValue, "wide")
    compare(result.schema[1].defaultValue, "x")
    compare(result.schema[2].options.length, 32)
  }

  function test_text_rows_bound_lengths_and_strip_controls() {
    var long = new Array(6000).join("y")
    var result = spec([
      row("a", "string", { maxLength: 4, placeholder: new Array(100).join("p"), defaultValue: "abcdefg" }),
      row("b", "string", { defaultValue: "line\nbreak" }),
      row("c", "path", { maxLength: 999999, defaultValue: long }),
      row("d", "string", { maxLength: 0 }),
      row("e", "string", { defaultValue: { nested: true } })
    ])
    compare(result.schema[0].maxLength, 4)
    compare(result.schema[0].placeholder.length, 64)
    compare(result.schema[0].defaultValue, "abcd")
    compare(result.schema[1].defaultValue, "linebreak")
    compare(result.schema[2].maxLength, 4096)
    compare(result.schema[2].defaultValue.length, 4096)
    compare(result.schema[3].maxLength, 1)
    compare(result.schema[4].defaultValue, "")
  }

  function test_boolean_rows_coerce_strings() {
    var result = spec([
      row("a", "boolean", { defaultValue: "false" }),
      row("b", "boolean", { defaultValue: 1 }),
      row("c", "boolean", { defaultValue: [] }),
      row("d", "boolean")
    ], { c: "true" })
    compare(result.defaults, { a: false, b: true, c: true, d: false })
  }

  function test_omarchy_bar_widget_schema_reads_unchanged() {
    var result = spec([
      { key: "interval", type: "integer", label: "Poll (s)", min: 5, max: 600, step: 5, defaultValue: 30, description: "How often to poll" },
      { key: "unit", type: "enum", label: "Unit", options: ["metric", "imperial"], defaultValue: "metric" },
      { key: "root", type: "path", label: "Root", defaultValue: "~/git" },
      { key: "compact", type: "boolean", label: "Compact", defaultValue: true }
    ])
    compare(keys(result), ["interval", "unit", "root", "compact"])
    compare(result.defaults, { interval: 30, unit: "metric", root: "~/git", compact: true })
    compare(result.schema[0].description, "How often to poll")
  }

  function test_type_zero_is_the_in_range_value_nearest_zero() {
    var result = spec([
      row("a", "number", { max: -2 }),
      row("b", "integer", { max: -1 }),
      row("c", "number", { min: 3 }),
      row("d", "integer", { min: 4, max: 9 }),
      row("e", "number", { min: -1, max: 1 })
    ])
    compare(result.defaults, { a: -2, b: -1, c: 3, d: 4, e: 0 })
    for (var i = 0; i < result.schema.length; i++) {
      verify(isFinite(result.schema[i].min), result.schema[i].key)
      verify(isFinite(result.schema[i].max), result.schema[i].key)
      verify(isFinite(result.schema[i].defaultValue), result.schema[i].key)
    }
  }

  function test_object_text_fields_fall_back() {
    var result = spec([
      row("a", "string", { label: { x: 1 }, description: ["d"], placeholder: null }),
      row("b", "enum", { options: [{ value: "v", label: { y: 2 } }, { value: "w", label: ["z"] }] })
    ])
    compare(result.schema[0].label, "a")
    compare(result.schema[0].description, "")
    compare(result.schema[0].placeholder, "")
    compare(result.schema[1].options, [{ value: "v", label: "v" }, { value: "w", label: "w" }])
  }

  function test_prototype_names_are_reserved_keys_but_ordinary_values() {
    var result = spec([
      row("constructor", "boolean"),
      row("toString", "string"),
      row("__proto__", "string"),
      row("hasOwnProperty", "string"),
      row("valueOf", "boolean"),
      row("toString_", "constructor"),
      row("constructor_", "enum", { options: ["constructor", "toString", "hasOwnProperty", "ok"] })
    ], { constructor_: "hasOwnProperty" })
    compare(keys(result), ["constructor_"])
    compare(result.schema[0].options.map(function(option) { return option.value }), ["constructor", "toString", "hasOwnProperty", "ok"])
    compare(result.defaults, { constructor_: "hasOwnProperty" })
    var probe = ({})
    probe.hasOwnProperty = "x"
    verify(Object.keys(probe).indexOf("hasOwnProperty") < 0)
  }

  function test_step_is_bounded_by_the_range() {
    var result = spec([
      row("a", "integer", { step: 1e300 }),
      row("b", "number", { step: 1e-320 }),
      row("c", "number", { min: 0, max: 1, step: 5 }),
      row("d", "integer", { min: 3, max: 3, step: 7 }),
      row("e", "number", { min: 0, max: 1e-12, step: 0.1 })
    ])
    compare(result.schema[0].step, 2e9)
    compare(result.schema[1].step, 0.1)
    compare(result.schema[2].step, 1)
    compare(result.schema[3].step, 7)
    compare(result.schema[4].step, 1e-12)
  }
}

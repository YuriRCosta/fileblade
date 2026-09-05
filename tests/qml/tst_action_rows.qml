import QtQuick
import QtTest
import "../../lib/ActionRows.js" as ActionRows

TestCase {
  name: "ActionRowsRegression"

  function entry(path, isDir) {
    return { path: path, isDir: !!isDir }
  }

  function raw(id, contexts, extra) {
    var result = { key: "kurt.tools/" + id, id: id, title: id, contexts: contexts }
    if (extra) for (var name in extra) result[name] = extra[name]
    return result
  }

  function row(id, contexts, extra) {
    return ActionRows.action(raw(id, contexts, extra))
  }

  function keys(rows) {
    return rows.map(function(value) { return value.key })
  }

  function test_context_for_follows_the_fixed_order() {
    compare(ActionRows.contextFor([entry("/tmp/a.txt", false)], "/tmp"), "file")
    compare(ActionRows.contextFor([entry("/tmp/folder", true)], "/tmp"), "dir")
    compare(ActionRows.contextFor([entry("/tmp/a", false), entry("/tmp/b", true)], "/tmp"), "selection")
    compare(ActionRows.contextFor([], "/tmp"), "root")
    compare(ActionRows.contextFor([], ""), "none")
    compare(ActionRows.contextFor(undefined, undefined), "none")
    compare(ActionRows.contextFor([{ path: "/tmp/a", is_dir: true }], ""), "dir")
  }

  function test_context_of_picks_the_first_declared_context_that_applies() {
    var selection = row("many", ["selection"])
    compare(ActionRows.contextOf(selection, [entry("/tmp/a", false)], "/tmp"), "selection")
    var folders = row("folders", ["dir", "root"])
    compare(ActionRows.contextOf(folders, [entry("/tmp/a", false)], "/tmp"), "root")
    compare(ActionRows.contextOf(folders, [entry("/tmp/a", false)], ""), "")
    var always = row("always", ["none"])
    compare(ActionRows.contextOf(always, [entry("/tmp/a", false)], "/tmp"), "none")
  }

  function test_matches_is_membership_only() {
    var action = row("one", ["file", "dir"])
    verify(ActionRows.matches(action, "file"))
    verify(!ActionRows.matches(action, "selection"))
    verify(!ActionRows.matches(null, "file"))
    verify(!ActionRows.matches(action, undefined))
  }

  function test_rows_for_filters_and_keeps_none_always() {
    var actions = [
      row("files", ["file"]),
      row("folders", ["dir"]),
      row("always", ["none"]),
      row("roots", ["root"])
    ]
    compare(keys(ActionRows.rowsFor(actions, [entry("/tmp/a.txt", false)], "/tmp")),
      ["kurt.tools/files", "kurt.tools/always", "kurt.tools/roots"])
    compare(keys(ActionRows.rowsFor(actions, [entry("/tmp/d", true)], "")),
      ["kurt.tools/folders", "kurt.tools/always"])
    compare(keys(ActionRows.rowsFor(actions, [], "")), ["kurt.tools/always"])
    compare(ActionRows.rowsFor(undefined, [], "").length, 0)
  }

  function test_rows_for_caps_at_sixty_four() {
    var actions = []
    for (var i = 0; i < 200; i++) actions.push(row("a" + i, ["none"]))
    compare(actions.length, 200)
    compare(ActionRows.rowsFor(actions, [], "").length, 64)
  }

  function test_action_rejects_unusable_rows_and_bounds_the_rest() {
    compare(ActionRows.action(null), null)
    compare(ActionRows.action([]), null)
    compare(ActionRows.action(raw("", ["file"])), null)
    compare(ActionRows.action(raw("dump", [])), null)
    compare(ActionRows.action(raw("dump", ["nowhere"])), null)
    compare(ActionRows.action({ key: "user/dump", id: "dump", title: "", contexts: ["file"] }), null)
    var bounded = ActionRows.action({
      key: "user/dump",
      id: "dump",
      title: "  Dump\u0000 env  ",
      description: "one\u202etwo",
      contexts: ["file", "file", "root", "bogus"],
      source: "user",
      output: "loud",
      timeout: 4000,
      glyph: ""
    })
    compare(bounded.title, "Dump env")
    compare(bounded.description, "onetwo")
    compare(bounded.contexts, ["file", "root"])
    compare(bounded.source, "user")
    compare(bounded.output, "notice")
    compare(bounded.timeout, 900)
    compare(bounded.glyph, ActionRows.DEFAULT_GLYPH)
    compare(bounded.confirm, false)
    compare(ActionRows.action(raw("dump", ["file"], { timeout: "x" })).timeout, 60)
    compare(ActionRows.action(raw("dump", ["file"], { timeout: 0 })).timeout, 1)
  }

  function test_normalize_drops_bad_rows_and_caps_the_list() {
    var rows = [raw("a", ["file"]), null, raw("", ["file"]), raw("b", ["none"])]
    compare(ActionRows.normalize(rows).length, 2)
    compare(ActionRows.normalize(undefined).length, 0)
    var many = []
    for (var i = 0; i < 90; i++) many.push(raw("a" + i, ["none"]))
    compare(ActionRows.normalize(many).length, 64)
  }

  function test_errors_are_bounded_and_drop_empty_rows() {
    var errors = ActionRows.errors([
      { source: "kurt.tools", error: "bad\nignored" },
      null,
      { source: "empty", error: "" }
    ])
    compare(errors, [{ source: "kurt.tools", error: "bad" }])
  }

  function test_targets_follow_the_context_without_partial_selections() {
    var entries = [entry("/tmp/a", false), entry("/tmp/b", true)]
    compare(ActionRows.targetsFor("file", [entries[0]], "/tmp"), ["/tmp/a"])
    compare(ActionRows.targetsFor("dir", [entries[1]], "/tmp"), ["/tmp/b"])
    compare(ActionRows.targetsFor("selection", entries, "/tmp"), ["/tmp/a", "/tmp/b"])
    compare(ActionRows.targetsFor("root", entries, "/tmp"), [])
    compare(ActionRows.targetsFor("none", entries, "/tmp"), [])
    compare(ActionRows.targetCount("root", [], "/tmp"), 1)
    compare(ActionRows.targetCount("none", entries, "/tmp"), 0)
    var many = []
    for (var i = 0; i < 300; i++) many.push(entry("/tmp/f" + i, false))
    compare(ActionRows.targetsFor("selection", many, ""), [])
    compare(ActionRows.targetCount("selection", many, ""), 300)
    compare(ActionRows.targetsFor("selection", ["/tmp/plain", { path: "" }], ""), ["/tmp/plain"])
  }

  function test_first_line_stops_at_the_newline_and_strips_controls() {
    compare(ActionRows.firstLine("done\nrest", 40), "done")
    compare(ActionRows.firstLine("  a\u0007b  \nrest", 40), "ab")
    compare(ActionRows.firstLine("abcdef", 3), "abc")
    compare(ActionRows.firstLine(undefined, 10), "")
  }
}

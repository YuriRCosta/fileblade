import QtQuick
import QtTest
import "../../lib/SearchQuery.js" as SearchQuery
import "../fixtures/search_queries.js" as Fixture

TestCase {
  name: "SearchGrammar"

  function flatFilters(spec) {
    var rows = []
    var keys = Object.keys(spec.filters).sort()
    for (var k = 0; k < keys.length; k++) {
      var bucket = spec.filters[keys[k]]
      for (var w = 0; w < bucket.wanted.length; w++) rows.push({ key: keys[k], value: bucket.wanted[w], negate: false })
      for (var x = 0; x < bucket.excluded.length; x++) rows.push({ key: keys[k], value: bucket.excluded[x], negate: true })
    }
    return rows
  }

  function test_shared_fixture_data() {
    var rows = []
    for (var i = 0; i < Fixture.QUERIES.length; i++) rows.push({ tag: Fixture.QUERIES[i].query, row: Fixture.QUERIES[i] })
    return rows
  }

  function test_shared_fixture(data) {
    var spec = SearchQuery.parse(data.row.query, ["type"], {})
    compare(spec.terms.length, data.row.terms.length, "term count for " + data.row.query)
    for (var i = 0; i < data.row.terms.length; i++) {
      compare(spec.terms[i].text, data.row.terms[i].text)
      compare(!!spec.terms[i].negate, data.row.terms[i].negate)
      compare(!!spec.terms[i].exact, data.row.terms[i].exact)
      compare(spec.terms[i].field || "", data.row.terms[i].field)
    }
    var filters = flatFilters(spec)
    compare(filters.length, data.row.filters.length, "filter count for " + data.row.query)
    for (var f = 0; f < data.row.filters.length; f++) {
      compare(filters[f].key, data.row.filters[f].key)
      compare(filters[f].value, data.row.filters[f].value)
      compare(filters[f].negate, data.row.filters[f].negate)
    }
  }

  function test_plain_search_prioritizes_filename_matches_without_losing_parents() {
    var rows = [
      { name: "project", depth: 0 },
      { name: "target", depth: 1 },
      { name: "Cargo.toml", depth: 1 },
      { name: "docs", depth: 1 },
      { name: "agents-guide.md", depth: 2 },
      { name: "AGENTS.md", depth: 1 }
    ]
    var result = SearchQuery.rankedTree(rows, SearchQuery.parse("ag", [], {}), function(row) {
      return { name: row.name, text: row.name }
    })
    compare(result.rows.map(function(row) { return row.name }).join(","),
      "project,AGENTS.md,docs,agents-guide.md,target,Cargo.toml")
    compare(result.matched, 4)
    compare(rows[1].name, "target", "Searching must not reorder the original tree")
  }

  function test_filter_only_search_preserves_the_selected_order() {
    var rows = [
      { name: "project", depth: 0, type: "folder" },
      { name: "zebra.md", depth: 1, type: "file" },
      { name: "alpha.md", depth: 1, type: "file" }
    ]
    var result = SearchQuery.rankedTree(rows, SearchQuery.parse("type:file", ["type"], {}), function(row) {
      return { name: row.name, text: row.name, fields: { type: row.type } }
    })
    compare(result.rows.map(function(row) { return row.name }).join(","), "project,zebra.md,alpha.md")
    compare(result.matched, 2)
  }

  function test_search_keeps_nested_extension_groups_together() {
    var spec = SearchQuery.parse("ag", [], {})
    var groups = [
      { path: ["USER", "one"], name: "agent" },
      { path: ["USER", "two"], name: "target" },
      { path: ["PROJECT", "three"], name: "AGENTS.md" }
    ]
    groups.forEach(function(group) { group.rank = SearchQuery.rank(spec, { name: group.name, text: group.name }) })
    compare(SearchQuery.rankedGroups(groups).map(function(group) { return group.path.join("/") }).join(","),
      "USER/one,USER/two,PROJECT/three")
    groups[2].name = "ag"
    groups[2].rank = SearchQuery.rank(spec, { name: "ag", text: "ag" })
    compare(SearchQuery.rankedGroups(groups).map(function(group) { return group.path.join("/") }).join(","),
      "PROJECT/three,USER/one,USER/two")
  }
}

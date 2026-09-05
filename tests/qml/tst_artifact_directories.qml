import QtQuick
import QtTest
import "../../ui"

TestCase {
  name: "ArtifactDirectories"
  property var reads: []
  property var watches: []
  property var cancelled: []
  property var directories: null

  Item {
    id: files
    property bool showHidden: false
    function makeRow(raw, depth) {
      return { path: raw.path, name: raw.name, isDir: !!raw.is_dir, kind: raw.kind || "File" }
    }
    function backendRequest(name, args, generation, callback) {
      var id = "read-" + reads.length
      reads.push({ id: id, name: name, args: args, generation: generation, callback: callback })
      return id
    }
    function backendSubscribe(paths, generation, event, ready, closed) {
      var id = "watch-" + watches.length
      watches.push({ id: id, paths: paths, generation: generation, event: event, ready: ready, closed: closed })
      return id
    }
    function cancelBackendRequest(id, generation, dispose) { cancelled.push({ id: id, dispose: dispose }) }
  }

  Component { id: component; ArtifactDirectories {} }

  function init() {
    reads = []; watches = []; cancelled = []; files.showHidden = false
    directories = createTemporaryObject(component, this, { files: files })
    verify(directories !== null)
  }
  function cleanup() { if (directories) directories.destroy(); directories = null }
  function open(paths) { directories.paths = paths; directories.reconcile(); directories.startRead() }
  function response(path, entries) { return { ok: true, results: [{ path: path, ok: true, entries: entries }] } }
  function finish(index, path, entries) { reads[index].callback(response(path, entries)) }

  function test_closed_folders_do_no_work_and_expansion_uses_shared_rows() {
    directories.reconcile(); directories.startRead()
    compare(reads.length, 0); compare(watches.length, 0)
    open(["/skills/example"])
    compare(reads[0].name, "children-batch")
    finish(0, "/skills/example", [{ path: "/skills/example/SKILL.md", name: "SKILL.md", kind: "Markdown" }])
    compare(directories.children("/skills/example")[0].detail, "Markdown")
    compare(watches[0].paths, ["/skills/example"])
    directories.reconcile(); directories.startRead()
    compare(reads.length, 1)
  }

  function test_watch_ack_and_events_replace_stale_reads() {
    open(["/skills/example"])
    watches[0].ready({ skipped: [] })
    verify(cancelled.some(function(value) { return value.id === "read-0" && value.dispose }))
    finish(0, "/skills/example", [{ path: "/skills/example/stale", name: "stale" }])
    compare(directories.children("/skills/example").length, 0)
    directories.startRead()
    finish(1, "/skills/example", [{ path: "/skills/example/new", name: "new" }])
    compare(directories.children("/skills/example")[0].name, "new")
    watches[0].event({ events: ["delete"] }); directories.startRead()
    finish(2, "/skills/example", [])
    compare(directories.children("/skills/example").length, 0)
    compare(watches.length, 1)
  }

  function test_collapse_releases_cache_reads_and_watch_then_reopens_fresh() {
    open(["/skills/example"])
    directories.paths = []; directories.reconcile()
    compare(Object.keys(directories.cache).length, 0)
    compare(directories.watch, null)
    finish(0, "/skills/example", [{ path: "/skills/example/late", name: "late" }])
    compare(directories.children("/skills/example").length, 0)
    open(["/skills/example"])
    compare(reads.length, 2); compare(watches.length, 2)
  }

  function test_native_children_are_distinct_and_outside_paths_are_refused() {
    open(["/skills/example"])
    finish(0, "/skills/example", [
      { path: "file:///skills/example/%FF", name: "\\xFF" },
      { path: "/skills/example/\uFFFD", name: "\uFFFD" },
      { path: "/skills/example/../outside", name: "outside" },
      { path: "relative", name: "relative" }
    ])
    var entries = directories.children("/skills/example")
    compare(entries.length, 2)
    compare(entries[0].path, "file:///skills/example/%FF")
    compare(entries[1].path, "/skills/example/\uFFFD")
  }

  function test_hidden_toggle_refreshes_without_discarding_expansion() {
    open(["/skills/example"]); finish(0, "/skills/example", [])
    files.showHidden = true; directories.startRead()
    compare(directories.wanted, ["/skills/example"])
    verify(reads[1].args.indexOf("--show-hidden") >= 0)
  }

  function test_failed_and_capped_folders_report_actionable_status() {
    open(["/skills/example"])
    reads[0].callback({ ok: false, error: "Permission denied" })
    compare(directories.error, "Permission denied")
    directories.refresh(); directories.startRead()
    var data = response("/skills/example", []); data.results[0].truncated = true
    reads[1].callback(data)
    verify(directories.error.indexOf("open it in Files") >= 0)
    directories.refresh(); directories.startRead(); finish(2, "/skills/example", [])
    compare(directories.error, "")
  }

  function test_directory_budget_deduplicates_mixed_paths_and_batches_work() {
    directories.maximumDirectories = 10
    var paths = []
    for (var i = 0; i < 12; i++) paths.push("/skills/" + i)
    paths.push("file:///skills/0")
    open(paths)
    compare(directories.wanted.length, 10)
    compare(directories.request.paths.length, 8)
    compare(directories.queue.length, 2)
    verify(directories.error.indexOf("collapse some") >= 0)
    compare(watches[0].paths.length, 10)
  }

  function test_removed_watch_is_rearmed_and_partial_failure_can_be_retried() {
    open(["/skills/example"])
    watches[0].event({ events: ["move_self"] })
    compare(watches.length, 2)
    watches[0].closed({ error: "late" })
    compare(directories.watchError, "")
    watches[1].ready({ skipped: ["/skills/example"] })
    verify(directories.error.indexOf("could not be watched") >= 0)
    directories.refresh(true)
    compare(watches.length, 3)
    watches[2].ready({ skipped: [] })
    compare(directories.watchError, "")
  }

  function test_hiding_and_destruction_discard_all_owned_callbacks() {
    open(["/skills/example"])
    directories.active = false; directories.reconcile()
    compare(directories.request, null); compare(directories.watch, null)
    verify(cancelled.every(function(value) { return value.dispose === true }))
    directories.active = true; directories.reconcile(); directories.startRead()
    directories.destroy(); wait(0); directories = null
    verify(cancelled.some(function(value) { return value.id === "read-1" && value.dispose }))
    verify(cancelled.some(function(value) { return value.id === "watch-1" && value.dispose }))
  }
}

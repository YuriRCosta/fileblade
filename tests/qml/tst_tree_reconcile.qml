import QtQuick
import QtTest
import "../../controllers"
import "../../lib/PathText.js" as PathText

TestCase {
  name: "TreeReconcile"
  property var tree: null
  property var reads: []
  property int replacing: 0
  property int countWhenReplacing: -1
  property int removedRows: 0
  property int insertedRows: 0
  property int movedRows: 0
  Component { id: component; TreeController {} }
  QtObject {
    id: files
    property bool stateReady: true
    property bool open: true
    property string rootPath: "/tree"
    property bool showHidden: false
    property bool gitEnabled: true
    property bool quickNavActive: false
    property string searchQuery: ""
    property var searchModel: null
    property var favoritesModel: null
    property var recentModel: null
    property string selectedPath: ""
    property var selectedPaths: []
    property var selectedEntries: []
    property string selectionAnchorPath: ""
    property int gitStatusPollIntervalMs: 0
    property string queuedGitMetadataReason: ""
    property bool watcherRunning: false
    property var treeSort: []
    property var treeFilter: ({})
    property var priorityColumns: []
    property bool trashMode: false
    property bool recentMode: false
    property bool drivesMode: false
    property string rootRecoveryOrigin: ""

    function rootName(path) { return PathText.name(path) }
    function normalizeRoot(path) { return path || "" }
    function pathWithin(path, root) { return PathText.within(path, root) }
    function scheduleWatcherRestart() {}
    function remappedPathList(paths, mappings) { return paths }
    function pathRemoved(path, removals) { return removals.indexOf(path) >= 0 }
    function treeRowsReplacing() { replacing++; countWhenReplacing = tree ? tree.model.count : -1 }
    function requestVisibleGitMetadataRefresh(reason) {}
    function makeRow(raw, depth) {
      return tree.treeRowSnapshot({ name: PathText.name(raw.path), path: raw.path,
        isDir: !!raw.is_dir, isSymlink: !!raw.is_symlink, gitIgnored: !!raw.git_ignored,
        gitDeleted: !!raw.is_deleted, gitStatus: String(raw.git_status || ""), size: raw.size === undefined ? -1 : raw.size,
        modified: String(raw.modified || ""), kind: raw.kind, depth: depth })
    }
    function entrySnapshot(row) {
      var snapshot = tree.treeRowSnapshot(row)
      var transient = ["depth", "expanded", "loaded", "loading", "error", "windowLoaded", "windowTotal", "relative", "nameSpans", "relativeSpans"]
      for (var i = 0; i < transient.length; i++) delete snapshot[transient[i]]
      return snapshot
    }
    function listingOrderArguments(limit) { return tree.listingOrderArguments(limit) }
    function windowLimitFor(paths) { return tree.windowLimitFor(paths) }
    function backendRequest(name, args, generation, callback) {
      var id = "read-" + reads.length
      reads.push({ id: id, name: name, args: args, generation: generation, callback: callback })
      return id
    }
    function cancelBackendRequest(id, generation) {}
  }

  function dir(path, extra) { return Object.assign({ path: path, is_dir: true }, extra || {}) }
  function file(path, extra) { return Object.assign({ path: path, size: 1, modified: "t1" }, extra || {}) }
  function response(path, entries, extra) { return Object.assign({ path: path, ok: true, entries: entries }, extra || {}) }
  function finish(index, results) { reads[index].callback({ ok: true, results: results }) }
  function paths() { var out = []; for (var i = 0; i < tree.model.count; i++) out.push(tree.model.get(i).path); return out }
  function row(path) { var i = tree.indexOfTreePath(path); return i >= 0 ? tree.model.get(i) : null }
  function resetCounters() { replacing = 0; countWhenReplacing = -1; removedRows = 0; insertedRows = 0; movedRows = 0 }

  function init() {
    reads = []
    tree = createTemporaryObject(component, this, { service: files })
    verify(tree !== null)
    tree.model.rowsRemoved.connect(function(parent, first, last) { removedRows += last - first + 1 })
    tree.model.rowsInserted.connect(function(parent, first, last) { insertedRows += last - first + 1 })
    tree.model.rowsMoved.connect(function() { movedRows++ })
    tree.resetTree()
    finish(0, [response("/tree", [dir("/tree/branch"), file("/tree/a.txt"), file("/tree/b.txt"), file("/tree/c.txt")])])
    tree.toggleDirectory(tree.indexOfTreePath("/tree/branch"))
    finish(1, [response("/tree/branch", [file("/tree/branch/inner.txt")])])
    compare(paths(), ["/tree", "/tree/branch", "/tree/branch/inner.txt", "/tree/a.txt", "/tree/b.txt", "/tree/c.txt"])
    resetCounters()
  }
  function cleanup() { if (tree) tree.destroy(); tree = null }

  function test_a_changed_file_updates_its_own_row_and_moves_nothing() {
    var revision = tree.treeStructureRevision
    var rows = tree.treeRowsRevision
    tree.reconcileDirectory("/tree", response("/tree", [dir("/tree/branch"), file("/tree/a.txt"), file("/tree/b.txt", { size: 9, modified: "t2", git_status: "M" }), file("/tree/c.txt")]))
    compare(paths(), ["/tree", "/tree/branch", "/tree/branch/inner.txt", "/tree/a.txt", "/tree/b.txt", "/tree/c.txt"])
    compare(removedRows, 0)
    compare(insertedRows, 0)
    compare(movedRows, 0)
    compare(replacing, 1)
    compare(row("/tree/b.txt").size, 9)
    compare(row("/tree/b.txt").gitStatus, "M")
    compare(tree.treeStructureRevision, revision)
    verify(tree.treeRowsRevision > rows)
    verify(row("/tree/branch").expanded)
  }

  function test_an_unchanged_listing_touches_nothing() {
    var revision = tree.treeStructureRevision
    var rows = tree.treeRowsRevision
    tree.reconcileDirectory("/tree", response("/tree", [dir("/tree/branch"), file("/tree/a.txt"), file("/tree/b.txt"), file("/tree/c.txt")]))
    compare(replacing, 0)
    compare(removedRows + insertedRows + movedRows, 0)
    compare(tree.treeStructureRevision, revision)
    compare(tree.treeRowsRevision, rows)
  }

  function test_new_and_deleted_files_change_only_their_rows() {
    var revision = tree.treeStructureRevision
    tree.reconcileDirectory("/tree", response("/tree", [dir("/tree/branch"), file("/tree/a.txt"), file("/tree/added.txt"), file("/tree/c.txt")]))
    compare(paths(), ["/tree", "/tree/branch", "/tree/branch/inner.txt", "/tree/a.txt", "/tree/added.txt", "/tree/c.txt"])
    compare(removedRows, 1)
    compare(insertedRows, 1)
    compare(movedRows, 0)
    verify(tree.treeStructureRevision > revision)
    verify(row("/tree/branch").expanded)
    compare(tree.indexOfTreePath("/tree/added.txt"), 4)
  }

  function test_a_reordered_listing_moves_rows_instead_of_rebuilding() {
    tree.reconcileDirectory("/tree", response("/tree", [file("/tree/a.txt"), file("/tree/b.txt"), file("/tree/c.txt"), dir("/tree/branch")], { windowed: true }))
    compare(paths(), ["/tree", "/tree/a.txt", "/tree/b.txt", "/tree/c.txt", "/tree/branch", "/tree/branch/inner.txt"])
    compare(removedRows, 0)
    compare(insertedRows, 0)
    verify(movedRows > 0)
    verify(row("/tree/branch").expanded)
  }

  function test_full_refresh_announces_before_clearing_the_model() {
    tree.refreshTree()
    compare(replacing, 1)
    compare(countWhenReplacing, 6)
    verify(tree.treeLoading)
    finish(2, [response("/tree", [dir("/tree/branch"), file("/tree/a.txt")])])
    verify(tree.treeLoading)
    tryVerify(function() { return reads.length >= 4 })
    finish(3, [response("/tree/branch", [file("/tree/branch/inner.txt")])])
    tryVerify(function() { return !tree.treeLoading })
    compare(paths(), ["/tree", "/tree/branch", "/tree/branch/inner.txt", "/tree/a.txt"])
  }
}

import QtQuick
import QtTest
import "../../controllers"
import "../../lib/PathText.js" as PathText

TestCase {
  name: "TreeBranches"
  property var tree: null
  property var reads: []
  property var cancelled: []
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
    property string selectedPath: "/tree/branch"
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
    function treeRowsReplacing() {}
    function requestVisibleGitMetadataRefresh(reason) {}
    function makeRow(raw, depth) {
      return tree.treeRowSnapshot({ name: PathText.name(raw.path), path: raw.path,
        isDir: !!raw.is_dir, isSymlink: !!raw.is_symlink, gitIgnored: !!raw.git_ignored,
        gitDeleted: !!raw.is_deleted, kind: raw.kind, depth: depth })
    }
    function listingOrderArguments(limit) { return tree.listingOrderArguments(limit) }
    function windowLimitFor(paths) { return tree.windowLimitFor(paths) }
    function backendRequest(name, args, generation, callback) {
      var id = "read-" + reads.length
      reads.push({ id: id, name: name, args: args, generation: generation, callback: callback })
      return id
    }
    function cancelBackendRequest(id, generation) { cancelled.push(id) }
    function openDefault() { fail("recursive arrows must never open a file") }
  }

  function dir(path, extra) { return Object.assign({ path: path, is_dir: true }, extra || {}) }
  function response(path, entries, extra) { return Object.assign({ path: path, ok: true, entries: entries }, extra || {}) }
  function finish(index, results) { reads[index].callback({ ok: true, results: results }) }
  function row(path) { var i = tree.indexOfTreePath(path); return i >= 0 ? tree.model.get(i) : null }
  function opened(path) { return !!row(path) && row(path).expanded }
  function waitReads(count) { tryVerify(function() { return reads.length >= count }) }

  function init() {
    reads = []; cancelled = []; files.open = true; files.rootPath = "/tree"
    files.selectedPath = "/tree/branch"
    tree = createTemporaryObject(component, this, { service: files })
    verify(tree !== null)
    tree.resetTree()
    finish(0, [response("/tree", [dir("/tree/branch"), dir("/tree/sibling"), { path: "/tree/file" }])])
  }
  function cleanup() { if (tree) tree.destroy(); tree = null }

  function test_lazy_recursion_keeps_captured_subtree_and_siblings_separate() {
    verify(tree.setBranchExpanded("/tree/branch", true)); waitReads(2)
    compare(reads[1].args[1], "/tree/branch")
    files.selectedPath = "/tree/sibling"
    finish(1, [response("/tree/branch", [dir("/tree/branch/inner")])])
    waitReads(3)
    compare(reads[2].args[1], "/tree/branch/inner")
    finish(2, [response("/tree/branch/inner", [{ path: "/tree/branch/inner/file" }])])
    tryVerify(function() { return row("/tree/branch/inner/file") !== null })
    verify(opened("/tree/branch")); verify(opened("/tree/branch/inner"))
    verify(!opened("/tree/sibling")); compare(files.rootPath, "/tree")
    wait(30); compare(reads.length, 3)
  }

  function test_recursive_collapse_forgets_descendants_but_keeps_siblings() {
    tree.toggleDirectory(tree.indexOfTreePath("/tree/sibling"))
    finish(1, [response("/tree/sibling", [dir("/tree/sibling/inner")])])
    tree.setBranchExpanded("/tree/branch", true); waitReads(3)
    finish(2, [response("/tree/branch", [dir("/tree/branch/inner")])]); waitReads(4)
    tree.restoreExpandedPaths = ["/tree/branch", "/tree/branch/inner", "/tree/sibling"]
    tree.setBranchExpanded("/tree/branch", false)
    finish(3, [response("/tree/branch/inner", [dir("/tree/branch/inner/late")])])
    verify(!opened("/tree/branch")); verify(opened("/tree/sibling"))
    compare(row("/tree/branch/inner"), null)
    compare(tree.restoreExpandedPaths, ["/tree/sibling"])
    tree.toggleDirectory(tree.indexOfTreePath("/tree/branch"))
    waitReads(5)
    finish(4, [response("/tree/branch", [dir("/tree/branch/inner")])])
    verify(!opened("/tree/branch/inner")); wait(30); compare(reads.length, 5)
  }

  function test_recursion_skips_directory_links_and_ignored_descendants() {
    tree.setBranchExpanded("/tree/branch", true); waitReads(2)
    finish(1, [response("/tree/branch", [dir("/tree/branch/link", { is_symlink: true }),
      dir("/tree/branch/ignored", { git_ignored: true }), dir("/tree/branch/normal")])])
    waitReads(3)
    compare(reads[2].args[1], "/tree/branch/normal")
    finish(2, [response("/tree/branch/normal", [])])
    wait(30); compare(reads.length, 3)
    verify(!opened("/tree/branch/link")); verify(!opened("/tree/branch/ignored"))
    tree.setBranchExpanded("/tree/branch/ignored", true); waitReads(4)
    finish(3, [response("/tree/branch/ignored", [dir("/tree/branch/ignored/child", { git_ignored: true })])])
    waitReads(5)
    compare(reads[4].args[1], "/tree/branch/ignored/child")
    finish(4, [response("/tree/branch/ignored/child", [])])
  }

  function test_more_pages_are_loaded_and_new_directories_expand() {
    tree.setBranchExpanded("/tree/branch", true); waitReads(2)
    finish(1, [response("/tree/branch", [{ path: "/tree/branch/first" }],
      { windowed: true, truncated: true, start: 0, total: 2 })])
    waitReads(3); compare(reads[2].name, "children-window")
    reads[2].callback(response("/tree/branch", [dir("/tree/branch/last")],
      { windowed: true, truncated: false, start: 1, total: 2 }))
    waitReads(4); compare(reads[3].args[1], "/tree/branch/last")
    finish(3, [response("/tree/branch/last", [])])
    compare(row("/tree/branch more"), null)
  }

  function test_file_and_deleted_folder_arrows_do_not_activate_anything() {
    verify(!tree.setBranchExpanded("/tree/file", true))
    verify(!tree.setBranchExpanded("/tree/file", false))
    tree.model.setProperty(tree.indexOfTreePath("/tree/branch"), "gitDeleted", true)
    verify(!tree.setBranchExpanded("/tree/branch", true))
    wait(30); compare(reads.length, 1)
  }

  function test_close_and_root_change_stop_recursion_data() {
    return [{ tag: "close", property: "open", value: false }, { tag: "navigate", property: "rootPath", value: "/elsewhere" }]
  }
  function test_close_and_root_change_stop_recursion(data) {
    tree.setBranchExpanded("/tree/branch", true); waitReads(2)
    files[data.property] = data.value
    finish(1, [response("/tree/branch", [dir("/tree/branch/late")])])
    wait(40); compare(reads.length, 2)
    verify(!opened("/tree/branch/late"))
  }

  function test_collapse_discards_queued_children_and_refuses_late_pages() {
    tree.setBranchExpanded("/tree/branch", true); waitReads(2)
    finish(1, [response("/tree/branch", [], { windowed: true, truncated: true, start: 0, total: 1 })])
    waitReads(3)
    tree.treeQueue = [{ path: "/tree/branch/child" }, { path: "/tree/sibling" }]
    tree.setBranchExpanded("/tree/branch", false)
    compare(tree.treeQueue, [{ path: "/tree/sibling" }])
    tree.treeQueue = []
    reads[2].callback(response("/tree/branch", [dir("/tree/branch/late")], { windowed: true }))
    wait(30); compare(row("/tree/branch/late"), null); compare(reads.length, 3)
  }
}

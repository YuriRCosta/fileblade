use std::{collections::HashSet, fs};

#[test]
fn ipc_service_calls_have_a_dispatch_target() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let service = fs::read_to_string(root.join("Service.qml")).unwrap();
    let ipc = fs::read_to_string(root.join("controllers/FileTreeIpc.qml")).unwrap();
    let definitions = regex::Regex::new(r"\b(?:function|signal)\s+(\w+)\s*\(").unwrap();
    let names: HashSet<_> = definitions
        .captures_iter(&service)
        .map(|m| m[1].to_owned())
        .collect();
    let calls = regex::Regex::new(r"\bservice\.(\w+)\s*\(").unwrap();
    for call in calls.captures_iter(&ipc) {
        assert!(
            names.contains(&call[1]),
            "missing Service.qml dispatch target: {}",
            &call[1]
        );
    }
}

#[test]
fn row_activation_uses_the_same_model_and_action_as_the_keyboard() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let row = fs::read_to_string(root.join("panes/BrowserRow.qml")).unwrap();
    let pane = fs::read_to_string(root.join("panes/TreePane.qml")).unwrap();
    assert!(row.contains("pane.activateIndex(ownerView, treeMode, index)"));
    assert!(!row.contains("controller.setRootPath(path)"));
    assert!(pane.contains("var model = view ? view.model"));
    assert!(
        pane.contains("function onTreeStructureRevisionChanged() { root.restoreTreeCursor() }")
    );
}

#[test]
fn focus_motion_is_not_measured_relative_to_an_animated_parent() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let slot = fs::read_to_string(root.join("blades/BladeSlot.qml")).unwrap();
    let surface = fs::read_to_string(root.join("blades/BladeSurface.qml")).unwrap();
    assert!(slot.contains("hoverPosition: hoverWatch.point.scenePosition"));
    assert!(slot.contains("host.focusedEdge === edge && host.focusedScreen === context.screen"));
    assert!(surface.contains("sheetHoverPosition: sheetHover.point.scenePosition"));
    assert!(!surface.contains("point.position"));
}

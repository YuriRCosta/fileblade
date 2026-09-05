use std::fs;
use std::path::Path;

#[test]
fn drag_source_owns_space_while_the_wheel_stays_pointer_pass_through() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let wheel = fs::read_to_string(root.join("ui/DropWheel.qml")).expect("read DropWheel.qml");
    let row = fs::read_to_string(root.join("panes/BrowserRow.qml")).expect("read BrowserRow.qml");
    let pane = fs::read_to_string(root.join("panes/TreePane.qml")).expect("read TreePane.qml");
    let controller = fs::read_to_string(root.join("controllers/DropWheelController.qml"))
        .expect("read DropWheelController.qml");
    let drag_keys = fs::read_to_string(root.join("controllers/DropWheelDragKeys.qml"))
        .expect("read DropWheelDragKeys.qml");

    assert!(wheel.contains(
        "WlrLayershell.keyboardFocus: wheelHere && !controller.dragActive && !controller.keyboardFocusReleased ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None"
    ));
    assert!(
        wheel.contains("mask: wheelHere && !controller.wheelFromDrag ? fullInput : passThrough")
    );
    assert!(row.contains(
        "if (pane.context) pane.context.requestFocus(\"\")\n    if (ownerView) ownerView.forceActiveFocus()"
    ));
    for handler in [
        "Keys.priority: Keys.BeforeItem\n    Keys.onPressed: function(event) { root.handleListKey(event, treeList, true) }",
        "Keys.priority: Keys.BeforeItem\n    Keys.onPressed: function(event) { root.handleListKey(event, searchList, false) }",
        "Keys.priority: Keys.BeforeItem\n    Keys.onPressed: function(event) { root.handleListKey(event, recentList, false) }",
    ] {
        assert!(pane.contains(handler));
    }
    assert!(
        controller.find("service.bladeHost.pressActive = true")
            < controller.find("dragActive = true"),
        "the blade must be guarded before keyboard capture reacts to the drag"
    );
    assert!(controller.contains("return dragKeys.handleRelease(event)"));
    assert!(drag_keys.contains("if (!event.isAutoRepeat) releaseTimer.restart()"));
    assert!(
        drag_keys
            .contains("return repeated || controller.wheelOpen || controller.modifierPressed()")
    );
    assert!(
        controller.contains(
            "if (withinHub(pointerX, pointerY)) resetHighlight()\n      else if (!activateAt(pointerX, pointerY)) close()"
        ),
        "releasing the drag on the hub keeps the wheel open for clicks and letters"
    );
}

#[test]
fn external_drop_actions_release_the_wheel_and_blade_before_the_backend_runs() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let wheel = fs::read_to_string(root.join("ui/DropWheel.qml")).expect("read DropWheel.qml");
    let controller = fs::read_to_string(root.join("controllers/DropWheelController.qml"))
        .expect("read DropWheelController.qml");
    let service = fs::read_to_string(root.join("Service.qml")).expect("read Service.qml");

    assert!(
        wheel.contains("focus: overlay.wheelHere && !overlay.controller.keyboardFocusReleased")
    );
    assert!(
        service.contains("if (dropWheelOpen) dropWheelController.keyboardFocusReleased = true")
    );
    assert!(service.contains("return bladeHost.yieldFocus()"));
    let run = controller
        .split("function run(actionId, placement, desktopId)")
        .nth(1)
        .and_then(|body| body.split("function finishRun").next())
        .expect("drop run body");
    assert!(run.contains("if (actionId === \"open\")"));
    assert!(run.contains(
        "service.navigateToLocation(String(directories[directories.length - 1]), null, \"browse\")"
    ));
    assert!(run.contains("paths = files"));
    assert!(run.contains("DropFocusPolicy.transfersFocus(actionId, fileCount)"));
    assert!(
        run.find("service.yieldFocusForExternalLaunch()").unwrap()
            < run.find("service.backendRequest(\"drop-run\"").unwrap()
    );
    let paste = controller
        .split("function pasteAt(targetScreen, x, y, form)")
        .nth(1)
        .and_then(|body| body.split("function finishPaste").next())
        .expect("drop paste body");
    assert!(
        paste.find("service.yieldFocusForExternalLaunch()").unwrap()
            < paste.find("service.backendRequest(\"drop-paste\"").unwrap()
    );
}

#[test]
fn wheel_shortcuts_and_results_belong_to_the_current_wheel() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let wheel = fs::read_to_string(root.join("ui/DropWheel.qml")).unwrap();
    let handler = wheel.split("function handleKey(event)").nth(1).unwrap();
    assert!(
        handler
            .find("controller.activateKey(event.text, repeated)")
            .unwrap()
            < handler.find("moveFor(event)").unwrap(),
        "displayed h/j/k/l shortcuts must win over optional letter navigation"
    );
    assert!(handler.contains("actionKeys.isRepeat(event)"));
    let controller = fs::read_to_string(root.join("controllers/DropWheelController.qml")).unwrap();
    assert!(controller.contains("if (!repeated) activateChild(i)\n          return true"));
    assert!(controller.contains("if (!repeated) activate(i)\n        return true"));
    let close = controller.split("function close()").nth(1).unwrap();
    assert!(close.contains("runGeneration++"));
    assert!(close.contains("runRequestId = \"\""));
    assert!(controller.contains("if (wheelOpen) close()"));
}

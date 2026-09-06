use serde_json::json;

fn monitors() -> Vec<serde_json::Value> {
    vec![
        json!({"id": 0, "name": "DP-1", "x": 0, "y": 0, "width": 2560, "height": 1440, "scale": 1.0, "transform": 0,
               "activeWorkspace": {"id": 1}, "specialWorkspace": {"id": 0}}),
        json!({"id": 1, "name": "HDMI-A-1", "x": 2560, "y": -80, "width": 3840, "height": 2160, "scale": 2.0, "transform": 0,
               "activeWorkspace": {"id": 5}, "specialWorkspace": {"id": -98}}),
        json!({"id": 2, "name": "DP-3", "x": -1080, "y": 0, "width": 1920, "height": 1080, "scale": 1.0, "transform": 1,
               "activeWorkspace": {"id": 7}, "specialWorkspace": {"id": 0}}),
    ]
}

#[test]
fn a_point_resolves_to_the_visible_workspace_of_the_output_that_contains_it() {
    let list = monitors();
    assert_eq!(
        fileblade::hyprland::visible_workspace_at(&list, 100, 100),
        Some(1)
    );
    assert_eq!(
        fileblade::hyprland::visible_workspace_at(&list, 2600, 0),
        Some(-98),
        "a shown special workspace wins on that output"
    );
    assert_eq!(
        fileblade::hyprland::visible_workspace_at(&list, 2560 + 1919, -80 + 1079),
        Some(-98),
        "scale 2 makes the 3840x2160 output 1920x1080 logical"
    );
    assert_eq!(
        fileblade::hyprland::visible_workspace_at(&list, 2560 + 1920, 0),
        None,
        "just past the scaled width"
    );
    assert_eq!(
        fileblade::hyprland::visible_workspace_at(&list, -500, 1500),
        Some(7),
        "rotated output swaps its size"
    );
    assert_eq!(
        fileblade::hyprland::visible_workspace_at(&list, -500, 1921),
        None
    );
    assert_eq!(
        fileblade::hyprland::visible_workspace_at(&list, 9000, 9000),
        None,
        "a gap resolves to nothing"
    );
}

#[test]
fn a_client_is_judged_against_its_own_monitor() {
    let list = monitors();
    assert_eq!(
        fileblade::hyprland::visible_workspace_for_monitor(&list, 1),
        Some(-98)
    );
    assert_eq!(
        fileblade::hyprland::visible_workspace_for_monitor(&list, 2),
        Some(7)
    );
    assert_eq!(
        fileblade::hyprland::visible_workspace_for_monitor(&list, 9),
        None
    );
}

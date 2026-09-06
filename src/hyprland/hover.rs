use super::*;

pub fn hover_target(blade_titles: &[String]) -> Value {
    let current = match cursor_position() {
        Ok(value) => value,
        Err(error) => return json!({"ok": false, "error": error.to_string()}),
    };
    let clients = match clients() {
        Ok(value) => value,
        Err(error) => return json!({"ok": false, "error": error.to_string()}),
    };
    let workspace_id = match workspace_visible_at_point(current.0, current.1) {
        Ok(value) => value,
        Err(error) => return json!({"ok": false, "error": error.to_string()}),
    };
    match window_under_cursor(&clients, current.0, current.1, workspace_id, blade_titles) {
        Some(target) => json!({
            "ok": true,
            "action": "hover-focus",
            "address": field_str(&target, "address"),
            "title": field_str(&target, "title"),
            "x": current.0,
            "y": current.1,
        }),
        None => json!({
            "ok": true,
            "action": "none",
            "x": current.0,
            "y": current.1,
        }),
    }
}

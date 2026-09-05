use super::*;

pub fn hypr_query(name: &str) -> AppResult<Value> {
    let response = hypr_request(&format!("j/{name}"))?;
    serde_json::from_slice(&response)
        .map_err(|_| AppError::command(format!("Hyprland query {name} returned invalid JSON")))
}

pub fn hypr_dispatch(expression: &str) -> AppResult<()> {
    let raw = hypr_request(&format!("dispatch {expression}"))?;
    let response = String::from_utf8_lossy(&raw).trim().to_string();
    if response.to_lowercase().starts_with("ok") {
        Ok(())
    } else {
        Err(AppError::command(if response.is_empty() {
            "Hyprland dispatcher returned no result".to_string()
        } else {
            response
        }))
    }
}

pub fn cursor_position() -> AppResult<(i64, i64)> {
    let value = hypr_query("cursorpos")?;
    Ok((field_i64(&value, "x"), field_i64(&value, "y")))
}

pub fn window_under_cursor(
    clients: &[Value],
    x: i64,
    y: i64,
    workspace_id: i64,
    excluded_titles: &[String],
) -> Option<Value> {
    let excluded: HashSet<&str> = excluded_titles.iter().map(String::as_str).collect();
    clients
        .iter()
        .filter(|client| client_hit(client, x, y, workspace_id, &excluded))
        .max_by_key(|client| {
            let floating = field_bool(client, "floating", false) as i64;
            let focus = field_i64(client, "focusHistoryID");
            (floating, -focus)
        })
        .cloned()
}

pub fn has_window_at_point(clients: &[Value], x: i64, y: i64, excluded_titles: &[String]) -> bool {
    let excluded: HashSet<&str> = excluded_titles.iter().map(String::as_str).collect();
    clients
        .iter()
        .any(|client| client_geometry_hit(client, x, y, &excluded))
}

pub fn active_window() -> Value {
    let value = match hypr_query("activewindow") {
        Ok(value) => value,
        Err(error) => {
            return json!({
                "ok": false,
                "address": "",
                "workspace": Value::Null,
                "error": error.to_string(),
            });
        }
    };
    if field_str(&value, "address").is_empty() {
        return json!({
            "ok": true,
            "address": "",
            "workspace": Value::Null,
            "class": "",
            "title": "",
        });
    }
    json!({
        "ok": true,
        "address": field_str(&value, "address"),
        "workspace": value.get("workspace").and_then(|workspace| workspace.get("id")).cloned().unwrap_or(Value::Null),
        "class": field_str(&value, "class"),
        "title": field_str(&value, "title"),
    })
}

pub(super) fn hypr_request(request: &str) -> AppResult<Vec<u8>> {
    if let Some(socket) = command_socket() {
        return socket_request(&socket, request);
    }
    let binary = which("hyprctl").ok_or_else(|| AppError::command("hyprctl is not installed"))?;
    let arguments = if let Some(name) = request.strip_prefix("j/") {
        std::iter::once("-j".to_string())
            .chain(name.split_ascii_whitespace().map(str::to_string))
            .collect()
    } else if let Some(expression) = request.strip_prefix("dispatch ") {
        vec!["dispatch".to_string(), expression.to_string()]
    } else {
        return Err(AppError::invalid("unsupported Hyprland request"));
    };
    let output = CommandSpec::new(binary)
        .args(arguments)
        .timeout(CONTROL_TIMEOUT)
        .limits(RESPONSE_LIMIT, 256 * 1024)
        .run()?;
    if output.stdout_truncated || output.stderr_truncated {
        return Err(AppError::command(
            "Hyprland response exceeded its byte limit",
        ));
    }
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(AppError::command(if error.is_empty() {
            "Hyprland request failed".to_string()
        } else {
            error
        }));
    }
    Ok(output.stdout)
}

pub(super) fn command_socket() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("FILEBLADE_HYPR_SOCKET") {
        let path = PathBuf::from(path);
        return path.is_absolute().then_some(path);
    }
    let signature = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").ok()?;
    if signature.is_empty() || signature.contains('/') || signature.contains("..") {
        return None;
    }
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(format!("/run/user/{}", rustix::process::getuid().as_raw()))
        });
    [
        runtime.join("hypr").join(&signature).join(".socket.sock"),
        PathBuf::from("/tmp")
            .join("hypr")
            .join(signature)
            .join(".socket.sock"),
    ]
    .into_iter()
    .find(|path| path.exists())
}

pub(super) fn socket_request(path: &Path, request: &str) -> AppResult<Vec<u8>> {
    let mut stream = UnixStream::connect(path)
        .map_err(|error| AppError::command(format!("could not connect to Hyprland: {error}")))?;
    stream.set_read_timeout(Some(CONTROL_TIMEOUT))?;
    stream.set_write_timeout(Some(CONTROL_TIMEOUT))?;
    stream.write_all(request.as_bytes())?;
    stream.shutdown(std::net::Shutdown::Write)?;
    let mut response = Vec::new();
    let mut chunk = [0_u8; 32 * 1024];
    loop {
        let count = stream.read(&mut chunk)?;
        if count == 0 {
            break;
        }
        if response.len() + count > RESPONSE_LIMIT {
            return Err(AppError::command("Hyprland response exceeded 4 MiB"));
        }
        response.extend_from_slice(&chunk[..count]);
    }
    Ok(response)
}

pub(super) fn client_hit(
    client: &Value,
    x: i64,
    y: i64,
    workspace_id: i64,
    excluded: &HashSet<&str>,
) -> bool {
    if client_workspace_id(client) != workspace_id {
        return false;
    }
    client_geometry_hit(client, x, y, excluded)
}

fn client_geometry_hit(client: &Value, x: i64, y: i64, excluded: &HashSet<&str>) -> bool {
    if !field_bool(client, "mapped", true)
        || field_bool(client, "hidden", false)
        || excluded.contains(field_str(client, "title").as_str())
    {
        return false;
    }
    let (left, top, width, height) = client_rect(client);
    left <= x && x < left + width && top <= y && y < top + height
}

pub(super) fn clients() -> AppResult<Vec<Value>> {
    hypr_query("clients")?
        .as_array()
        .cloned()
        .ok_or_else(|| AppError::command("Hyprland clients response was not a list"))
}

pub(super) fn client_by_address<'a>(clients: &'a [Value], address: &str) -> Option<&'a Value> {
    clients
        .iter()
        .find(|client| field_str(client, "address") == address)
}

pub(super) fn client_workspace_id(client: &Value) -> i64 {
    client
        .get("workspace")
        .map(|workspace| field_i64(workspace, "id"))
        .unwrap_or(-10_000)
}

pub(super) fn client_rect(client: &Value) -> (i64, i64, i64, i64) {
    let at = field_pair(client, "at");
    let size = field_pair(client, "size");
    (at.0, at.1, size.0, size.1)
}

pub(super) fn monitor_name(monitor_id: Value) -> String {
    let id = monitor_id.as_i64().unwrap_or(-1);
    hypr_query("monitors")
        .ok()
        .and_then(|value| value.as_array().cloned())
        .and_then(|monitors| {
            monitors
                .into_iter()
                .find(|monitor| field_i64(monitor, "id") == id)
        })
        .map(|monitor| field_str(&monitor, "name"))
        .unwrap_or_default()
}

pub(super) fn field_pair(value: &Value, key: &str) -> (i64, i64) {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|parts| {
            (
                parts.first().and_then(Value::as_i64).unwrap_or(0),
                parts.get(1).and_then(Value::as_i64).unwrap_or(0),
            )
        })
        .unwrap_or((0, 0))
}

pub(super) fn field_str(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

pub(super) fn field_i64(value: &Value, key: &str) -> i64 {
    value.get(key).and_then(Value::as_i64).unwrap_or(-1)
}

pub(super) fn field_bool(value: &Value, key: &str, default: bool) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_probe_finds_visible_clients_without_a_workspace_query() {
        let clients = vec![json!({
            "address": "0x1",
            "title": "Terminal",
            "mapped": true,
            "hidden": false,
            "at": [100, 200],
            "size": [500, 400],
            "workspace": {"id": 7},
        })];

        assert!(has_window_at_point(&clients, 200, 300, &[]));
        assert!(!has_window_at_point(
            &clients,
            200,
            300,
            &["Terminal".to_string()]
        ));
        assert!(!has_window_at_point(&clients, 50, 300, &[]));
    }
}

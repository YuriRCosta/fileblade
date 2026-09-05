use super::*;

pub(super) fn applications(path: String) -> AppResult<PublicResult> {
    let result = backend_json(&[
        "applications".to_string(),
        "--path".to_string(),
        absolute_path(&path),
    ])?;
    if !result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        return Err(AppError::command(error_text(
            &result,
            &format!("Could not list applications for {path}"),
        )));
    }
    Ok(PublicResult::one(result))
}

pub(super) fn open(options: OpenArgs) -> AppResult<PublicResult> {
    let response = if let Some(path) = &options.path {
        ipc("openPath", std::slice::from_ref(path))?
    } else {
        ipc("openSelection", &[])?
    };
    launch_response(
        response,
        options.path.as_deref().unwrap_or(""),
        options.wait,
    )
}

pub(super) fn launch_method(
    method: &str,
    path: String,
    wait: WaitTenArgs,
) -> AppResult<PublicResult> {
    launch_response(ipc(method, std::slice::from_ref(&path))?, &path, wait)
}

pub(super) fn open_with(options: OpenWithArgs) -> AppResult<PublicResult> {
    let path = absolute_path(&options.path);
    let response = ipc("openWithPath", &[path, options.desktop_id])?;
    launch_response(response, &options.path, options.wait)
}

pub(super) fn launch_response(
    response: String,
    path: &str,
    wait: WaitTenArgs,
) -> AppResult<PublicResult> {
    if !wait.wait || response != "queued" {
        return Ok(PublicResult::one(response));
    }
    let expected = (!path.is_empty()).then(|| absolute_path(path));
    let result = poll(
        wait.timeout,
        || {
            let state = object_response("status", &[])?;
            let complete = !state
                .get("launchBusy")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let matches = expected
                .as_ref()
                .is_none_or(|path| text_field(&state, "lastLaunchedPath") == *path);
            if !complete || !matches {
                return Ok(None);
            }
            if nonempty(&state, "launchError") && !nonempty(&state, "launchStatus") {
                return Err(AppError::command(text_field(&state, "launchError")));
            }
            Ok(Some(Value::Object(state)))
        },
        "Timed out waiting for a window",
    )?;
    Ok(PublicResult::many(vec![Value::String(response), result]))
}

pub(super) fn point(at: Option<&str>) -> AppResult<(i64, i64)> {
    if let Some(raw) = at {
        let parts: Vec<&str> = raw.split(',').map(str::trim).collect();
        if parts.len() != 2 {
            return Err(AppError::invalid("--at expects x,y screen coordinates"));
        }
        let x = parts[0]
            .parse::<i64>()
            .map_err(|_| AppError::invalid("--at expects x,y screen coordinates"))?;
        let y = parts[1]
            .parse::<i64>()
            .map_err(|_| AppError::invalid("--at expects x,y screen coordinates"))?;
        return Ok((x, y));
    }
    crate::hyprland::cursor_position()
        .map_err(|_| AppError::command("Could not read the cursor position from Hyprland"))
}

pub(super) fn chosen_paths(paths: &[String]) -> AppResult<Vec<String>> {
    let chosen: Vec<String> = if paths.is_empty() {
        selection_paths()?
    } else {
        paths.iter().map(|path| absolute_path(path)).collect()
    };
    if chosen.is_empty() {
        Err(AppError::invalid("No selection and no paths given"))
    } else {
        Ok(chosen)
    }
}

pub(super) fn drop_context(paths: &[String], at: Option<&str>) -> AppResult<Value> {
    let chosen = chosen_paths(paths)?;
    let (x, y) = point(at)?;
    let mut command = vec![
        "drop-context".to_string(),
        "--x".to_string(),
        x.to_string(),
        "--y".to_string(),
        y.to_string(),
    ];
    append_paths(&mut command, &chosen);
    let result = backend_json(&command)?;
    if !result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        return Err(AppError::command(error_text(
            &result,
            "Could not resolve the drop target",
        )));
    }
    Ok(result)
}

pub(super) fn wheel(options: PointPathsArgs) -> AppResult<PublicResult> {
    if !options.paths.is_empty() {
        select_paths(&options.paths)?;
    }
    let (x, y) = point(options.at.as_deref())?;
    let response = ipc("showDropWheel", &[x.to_string(), y.to_string()])?;
    if response != "open" {
        return Err(AppError::command(response));
    }
    Ok(PublicResult::one(response))
}

pub(super) fn paste_path(options: PastePathArgs) -> AppResult<PublicResult> {
    let chosen = chosen_paths(&options.paths)?;
    let (x, y) = point(options.at.as_deref())?;
    let mut command = vec![
        "drop-paste".to_string(),
        "--x".to_string(),
        x.to_string(),
        "--y".to_string(),
        y.to_string(),
        "--form".to_string(),
        if options.relative {
            "relative"
        } else {
            "absolute"
        }
        .to_string(),
    ];
    append_paths(&mut command, &chosen);
    if options.dry_run {
        command.push("--dry-run".to_string());
    }
    let result = backend_json(&command)?;
    if !result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        return Err(AppError::command(error_text(&result, "Paste failed")));
    }
    Ok(PublicResult::one(result))
}

pub(super) fn drop_actions(options: PointPathsArgs) -> AppResult<PublicResult> {
    Ok(PublicResult::one(drop_context(
        &options.paths,
        options.at.as_deref(),
    )?))
}

pub(super) fn drop_run(options: DropRunArgs) -> AppResult<PublicResult> {
    let context = drop_context(&options.paths, options.at.as_deref())?;
    let target = serde_json::to_string(context.get("target").unwrap_or(&Value::Null))?;
    let paths: Vec<String> = context
        .pointer("/files/paths")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect();
    let mut command = vec![
        "drop-run".to_string(),
        "--action".to_string(),
        options.action,
        "--placement".to_string(),
        options.placement,
        "--target".to_string(),
        target,
        "--desktop-id".to_string(),
        options.desktop_id,
    ];
    if options.dry_run {
        command.push("--dry-run".to_string());
    }
    append_paths(&mut command, &paths);
    let result = backend_json(&command)?;
    if !result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        return Err(AppError::command(error_text(&result, "Drop action failed")));
    }
    Ok(PublicResult::one(result))
}

pub(super) fn append_paths(command: &mut Vec<String>, paths: &[String]) {
    for path in paths {
        command.extend(["--path".to_string(), path.clone()]);
    }
}

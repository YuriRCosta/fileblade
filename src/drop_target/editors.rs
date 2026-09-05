use super::*;

pub(super) fn run_tmux(
    runner: &mut Runner,
    placement: &str,
    target: &Value,
    cwd: &str,
    action_command: &[String],
) -> AppResult<Value> {
    let terminal = target.get("terminal").unwrap_or(&Value::Null);
    let mut command = tmux_command(&text_field(terminal, "socket"));
    let session = text_field(terminal, "session");
    let cwd = cwd.to_string();
    let script = (!action_command.is_empty()).then(|| quoted_paths(action_command));
    match placement {
        "window" => {
            command.extend([
                "new-window".to_string(),
                "-t".to_string(),
                session,
                "-c".to_string(),
                cwd,
            ]);
            if let Some(script) = script {
                command.push(script);
            }
        }
        "session" => {
            let name = format!(
                "drop-{}",
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
                    % 100_000
            );
            let mut create = command.clone();
            create.extend([
                "new-session".to_string(),
                "-d".to_string(),
                "-s".to_string(),
                name.clone(),
                "-c".to_string(),
                cwd,
            ]);
            if let Some(script) = script {
                create.push(script);
            }
            let (code, _, error) = runner.capture(native_tmux_arguments(create)?, None)?;
            if code != 0 {
                return Ok(json!({"ok": false, "error": error.trim()}));
            }
            command.extend([
                "switch-client".to_string(),
                "-c".to_string(),
                text_field(terminal, "tty"),
                "-t".to_string(),
                name,
            ]);
        }
        _ => {
            command.push("split-window".to_string());
            match placement {
                "right" => command.push("-h".to_string()),
                "down" => command.push("-v".to_string()),
                _ => {}
            }
            command.extend(["-t".to_string(), session, "-c".to_string(), cwd]);
            if let Some(script) = script {
                command.push(script);
            }
        }
    }
    let (code, _, error) = runner.capture(native_tmux_arguments(command)?, None)?;
    Ok(json!({"ok": code == 0, "error": error.trim()}))
}

pub(super) fn run_nvim(
    runner: &mut Runner,
    placement: &str,
    target: &Value,
    facts: &Value,
) -> AppResult<Value> {
    let server = target
        .get("editor")
        .and_then(|editor| editor.get("server"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    if server.is_empty() {
        return Ok(json!({"ok": false, "error": "This nvim window has no server socket"}));
    }
    let files = string_array(facts, "files");
    let expression = nvim_open_expression(placement, &files)?;
    let (code, _, error) = runner.capture(
        vec![
            OsString::from("nvim"),
            OsString::from("--server"),
            if server.starts_with("file://") {
                parse_path(server)?.into_os_string()
            } else {
                server.into()
            },
            OsString::from("--remote-expr"),
            expression.into(),
        ],
        None,
    )?;
    if code == 0 {
        focus_target(runner, target)?;
    }
    Ok(json!({"ok": code == 0, "error": error.trim()}))
}

pub(super) fn native_tmux_arguments(arguments: Vec<String>) -> std::io::Result<Vec<OsString>> {
    arguments
        .iter()
        .enumerate()
        .map(|(index, value)| {
            if index > 0
                && matches!(arguments[index - 1].as_str(), "-c" | "-S")
                && value.starts_with("file://")
            {
                parse_path(value).map(PathBuf::into_os_string)
            } else {
                Ok(OsString::from(value))
            }
        })
        .collect()
}

pub(super) fn nvim_open_expression(placement: &str, files: &[String]) -> AppResult<String> {
    const BUFFER: &str = "(function(paths) local function drop(path, split, tab) local buffer = vim.fn.bufadd(path); local windows = vim.fn.win_findbuf(buffer); if #windows > 0 then vim.api.nvim_set_current_win(windows[1]) else if tab then vim.cmd.tabnew() elseif split or (vim.bo.modified and not vim.o.hidden) then vim.cmd.vsplit() end; vim.api.nvim_set_current_buf(buffer) end end; for _, path in ipairs(paths) do vim.fn.bufadd(path) end; if paths[1] ~= nil then drop(paths[1], false, false) end; return 0; end)(_A)";
    const SPLIT: &str = "(function(paths) local function drop(path, split, tab) local buffer = vim.fn.bufadd(path); local windows = vim.fn.win_findbuf(buffer); if #windows > 0 then vim.api.nvim_set_current_win(windows[1]) else if tab then vim.cmd.tabnew() elseif split or (vim.bo.modified and not vim.o.hidden) then vim.cmd.vsplit() end; vim.api.nvim_set_current_buf(buffer) end end; for index, path in ipairs(paths) do drop(path, index > 1, false) end; return 0; end)(_A)";
    const TAB: &str = "(function(paths) local function drop(path, split, tab) local buffer = vim.fn.bufadd(path); local windows = vim.fn.win_findbuf(buffer); if #windows > 0 then vim.api.nvim_set_current_win(windows[1]) else if tab then vim.cmd.tabnew() elseif split or (vim.bo.modified and not vim.o.hidden) then vim.cmd.vsplit() end; vim.api.nvim_set_current_buf(buffer) end end; for _, path in ipairs(paths) do drop(path, false, true) end; return 0; end)(_A)";

    let script = match placement {
        "buffer" => BUFFER,
        "split" => SPLIT,
        _ => TAB,
    };
    let paths = files
        .iter()
        .map(|path| parse_path(path))
        .collect::<std::io::Result<Vec<_>>>()?;
    let payload = paths
        .iter()
        .map(|path| {
            format!(
                "\"{}\"",
                path.as_os_str()
                    .as_bytes()
                    .iter()
                    .map(|byte| format!("\\x{byte:02x}"))
                    .collect::<String>()
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    Ok(format!("luaeval('{script}', [{payload}])"))
}

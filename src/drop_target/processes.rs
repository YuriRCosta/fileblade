use super::*;

#[derive(Clone, Debug)]
pub(super) struct ProcessRow {
    pub(super) pid: u32,
    pub(super) parent: u32,
    pub(super) comm: String,
    pub(super) arguments: Vec<String>,
}

pub(super) fn process_descendants(root_pid: u32) -> Vec<ProcessRow> {
    let started = Instant::now();
    let mut rows = Vec::new();
    let Ok(entries) = fs::read_dir("/proc") else {
        return rows;
    };
    for entry in entries.flatten().take(MAX_PROCESSES) {
        if started.elapsed() > Duration::from_millis(50) {
            break;
        }
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|value| value.parse::<u32>().ok())
        else {
            continue;
        };
        let stat = bounded_file_text(&entry.path().join("stat"));
        let Some(end) = stat.rfind(')') else {
            continue;
        };
        let Some(parent) = stat[end + 1..]
            .split_whitespace()
            .nth(1)
            .and_then(|value| value.parse::<u32>().ok())
        else {
            continue;
        };
        rows.push(ProcessRow {
            pid,
            parent,
            comm: String::new(),
            arguments: Vec::new(),
        });
    }
    if !rows.iter().any(|row| row.pid == root_pid) {
        rows.push(ProcessRow {
            pid: root_pid,
            parent: 0,
            comm: String::new(),
            arguments: Vec::new(),
        });
    }
    let mut children: HashMap<u32, Vec<u32>> = HashMap::new();
    for row in &rows {
        children.entry(row.parent).or_default().push(row.pid);
    }
    let mut descendants = HashSet::from([root_pid]);
    let mut pending = VecDeque::from([root_pid]);
    while let Some(parent) = pending.pop_front() {
        for child in children.get(&parent).into_iter().flatten() {
            if descendants.insert(*child) {
                pending.push_back(*child);
            }
        }
    }
    rows.into_iter()
        .filter(|row| descendants.contains(&row.pid))
        .map(|mut row| {
            row.comm = bounded_file_text(&PathBuf::from(format!("/proc/{}/comm", row.pid)))
                .trim()
                .to_string();
            row.arguments =
                bounded_file_bytes(&PathBuf::from(format!("/proc/{}/cmdline", row.pid)))
                    .split(|byte| *byte == 0)
                    .filter(|value| !value.is_empty())
                    .map(|value| {
                        let path = Path::new(OsStr::from_bytes(value));
                        if path.is_absolute() {
                            path_text(path)
                        } else {
                            command_text(path.as_os_str())
                        }
                    })
                    .collect();
            row
        })
        .collect()
}

pub(super) fn environment_value(processes: &[ProcessRow], key: &str) -> String {
    for row in processes.iter().take(128) {
        let bytes = bounded_file_bytes(&PathBuf::from(format!("/proc/{}/environ", row.pid)));
        for item in bytes.split(|byte| *byte == 0) {
            if let Some(value) = item.strip_prefix(format!("{key}=").as_bytes()) {
                let value = if key == "TMUX" {
                    value.split(|byte| *byte == b',').next().unwrap_or_default()
                } else {
                    value
                };
                if matches!(key, "HERDR_SOCKET_PATH" | "TMUX") && value.starts_with(b"/") {
                    return path_text(Path::new(OsStr::from_bytes(value)));
                }
                return std::str::from_utf8(value).unwrap_or_default().to_string();
            }
        }
    }
    String::new()
}

pub(super) fn nvim_server(processes: &[ProcessRow]) -> String {
    let runtime = std::env::var("XDG_RUNTIME_DIR")
        .unwrap_or_else(|_| format!("/run/user/{}", rustix::process::getuid().as_raw()));
    for row in processes {
        if row.comm != "nvim" {
            continue;
        }
        if let Some(index) = row
            .arguments
            .iter()
            .position(|argument| argument == "--listen")
            && let Some(path) = row
                .arguments
                .get(index + 1)
                .filter(|path| parse_path(path).is_ok_and(|path| path.exists()))
        {
            return path.clone();
        }
        let candidate = format!("{runtime}/nvim.{}.0", row.pid);
        if Path::new(&candidate).exists() {
            return candidate;
        }
    }
    String::new()
}

pub(super) fn herdr_socket(processes: &[ProcessRow]) -> String {
    let inherited = environment_value(processes, "HERDR_SOCKET_PATH");
    if !inherited.is_empty() {
        return inherited;
    }
    let session = processes
        .iter()
        .filter(|row| row.comm == "herdr")
        .find_map(|row| session_argument(&row.arguments));
    let Some(session) = session else {
        return String::new();
    };
    let fallback = expanded_path(&format!("~/.config/herdr/sessions/{session}/herdr.sock"));
    herdr_session_sockets()
        .remove(&session)
        .unwrap_or_else(|| path_text(&fallback))
}

pub(super) fn herdr_process_context(processes: &[ProcessRow]) -> Value {
    let socket = herdr_socket(processes);
    let pane_id = environment_value(processes, "HERDR_PANE_ID");
    if pane_id.is_empty() {
        return herdr_focused_pane(&socket);
    }
    json!({
        "workspace_id": environment_value(processes, "HERDR_WORKSPACE_ID"),
        "tab_id": environment_value(processes, "HERDR_TAB_ID"),
        "pane_id": pane_id,
        "socket": socket,
    })
}

pub(super) fn session_argument(arguments: &[String]) -> Option<String> {
    for (index, argument) in arguments.iter().enumerate() {
        if let Some(value) = argument.strip_prefix("--session=") {
            return Some(value.to_string());
        }
        if argument == "--session" {
            return arguments.get(index + 1).cloned();
        }
    }
    None
}

pub(super) fn herdr_session_sockets() -> HashMap<String, String> {
    let Some(binary) = which("herdr") else {
        return HashMap::new();
    };
    let output = CommandSpec::new(binary)
        .args(["session", "list"])
        .timeout(CONTROL_TIMEOUT)
        .limits(256 * 1024, 64 * 1024)
        .run();
    let Ok(output) = output else {
        return HashMap::new();
    };
    output
        .stdout
        .split(|byte| *byte == b'\n')
        .skip(1)
        .filter_map(|line| {
            let fields = line
                .split(u8::is_ascii_whitespace)
                .filter(|field| !field.is_empty())
                .collect::<Vec<_>>();
            let socket = *fields.last()?;
            if fields.len() < 4 || !socket.starts_with(b"/") || !socket.ends_with(b".sock") {
                return None;
            }
            Some((
                std::str::from_utf8(fields[0]).ok()?.to_string(),
                path_text(Path::new(OsStr::from_bytes(socket))),
            ))
        })
        .collect()
}

pub(super) fn herdr_focused_pane(socket: &str) -> Value {
    let Some(binary) = which("herdr") else {
        return json!({});
    };
    let workspace = herdr_query(&binary, &["workspace", "list"], socket)
        .and_then(|query| query.run())
        .ok()
        .and_then(|output| serde_json::from_slice::<Value>(&output.stdout).ok())
        .and_then(|value| focused_row(value.pointer("/result/workspaces")))
        .unwrap_or(Value::Null);
    let workspace_id = text_field(&workspace, "workspace_id");
    if workspace_id.is_empty() {
        return json!({});
    }
    let pane = herdr_query(
        &binary,
        &["pane", "list", "--workspace", &workspace_id],
        socket,
    )
    .and_then(|query| query.run())
    .ok()
    .and_then(|output| serde_json::from_slice::<Value>(&output.stdout).ok())
    .and_then(|value| focused_row(value.pointer("/result/panes")))
    .unwrap_or(Value::Null);
    json!({
        "workspace_id": pane.get("workspace_id").and_then(Value::as_str).unwrap_or(&workspace_id),
        "tab_id": text_field(&pane, "tab_id"),
        "pane_id": text_field(&pane, "pane_id"),
        "cwd": pane.get("foreground_cwd").and_then(Value::as_str).filter(|value| !value.is_empty()).unwrap_or_else(|| pane.get("cwd").and_then(Value::as_str).unwrap_or_default()),
        "agent": text_field(&pane, "agent"),
        "socket": socket,
    })
}

fn herdr_query(binary: &Path, arguments: &[&str], socket: &str) -> AppResult<CommandSpec> {
    let query = CommandSpec::new(binary)
        .args(arguments.iter().copied())
        .timeout(CONTROL_TIMEOUT)
        .limits(256 * 1024, 64 * 1024);
    if socket.is_empty() {
        Ok(query)
    } else {
        Ok(query.env("HERDR_SOCKET_PATH", parse_path(socket)?))
    }
}

pub(super) fn tmux_client(processes: &[ProcessRow]) -> Value {
    if which("tmux").is_none() {
        return json!({});
    }
    let pids: HashSet<u32> = processes.iter().map(|row| row.pid).collect();
    let socket = environment_value(processes, "TMUX");
    let Ok(command) = native_tmux_arguments(tmux_command(&socket)) else {
        return json!({});
    };
    let output = CommandSpec::new(command[0].clone())
        .args(command.iter().skip(1))
        .args([
            "list-clients".to_string(),
            "-F".to_string(),
            "#{client_pid}\t#{client_tty}\t#{session_id}\t#{pane_pid}".to_string(),
        ])
        .timeout(CONTROL_TIMEOUT)
        .limits(256 * 1024, 64 * 1024)
        .run();
    let Ok(output) = output else {
        return json!({});
    };
    for line in std::str::from_utf8(&output.stdout)
        .unwrap_or_default()
        .lines()
    {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() == 4
            && fields[0]
                .parse::<u32>()
                .ok()
                .is_some_and(|pid| pids.contains(&pid))
        {
            return json!({
                "client_pid": fields[0].parse::<u32>().unwrap_or_default(),
                "tty": fields[1],
                "session": fields[2],
                "cwd": fields[3].parse::<u32>().ok().and_then(|pid| fs::read_link(format!("/proc/{pid}/cwd")).ok()).map(|path| path_text(&path)).unwrap_or_default(),
                "socket": socket,
            });
        }
    }
    json!({})
}

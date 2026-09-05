use crate::command::which;
use crate::common::{expanded_path, parse_path, path_text};
use crate::filesystem::read_regular_prefix;
use serde_json::{Value, json};
use std::path::PathBuf;

struct Agent {
    id: &'static str,
    label: &'static str,
    binaries: &'static [&'static str],
    mise: &'static [&'static str],
    homes: &'static [&'static str],
}

const AGENTS: [Agent; 6] = [
    Agent {
        id: "claude-code",
        label: "Claude Code",
        binaries: &["claude"],
        mise: &["claude"],
        homes: &["~/.claude"],
    },
    Agent {
        id: "codex",
        label: "Codex",
        binaries: &["codex"],
        mise: &["codex"],
        homes: &["~/.codex"],
    },
    Agent {
        id: "opencode",
        label: "OpenCode",
        binaries: &["opencode"],
        mise: &["opencode"],
        homes: &["~/.config/opencode"],
    },
    Agent {
        id: "pi",
        label: "Pi",
        binaries: &["pi"],
        mise: &["pi"],
        homes: &["~/.pi"],
    },
    Agent {
        id: "copilot-cli",
        label: "GitHub Copilot CLI",
        binaries: &["copilot"],
        mise: &["copilot"],
        homes: &["~/.copilot"],
    },
    Agent {
        id: "antigravity",
        label: "Google Antigravity",
        binaries: &["antigravity"],
        mise: &["aqua-google-antigravity-antigravity-cli", "antigravity"],
        homes: &["~/.gemini/antigravity", "~/.antigravity"],
    },
];

pub fn is_stub(raw_path: &str) -> bool {
    let Ok(path) = parse_path(raw_path) else {
        return true;
    };
    let resolved = path.canonicalize().unwrap_or(path);
    read_regular_prefix(&resolved, 1024)
        .map(|head| {
            head.starts_with(b"#!")
                && [
                    b"mise use".as_slice(),
                    b"mise exec".as_slice(),
                    b"mise x ".as_slice(),
                ]
                .iter()
                .any(|marker| head.windows(marker.len()).any(|window| window == *marker))
        })
        .unwrap_or(false)
}

pub fn first_binary(names: &[&str]) -> String {
    names
        .iter()
        .find_map(|name| {
            let found = which(name)?;
            (!is_stub(&path_text(&found))).then(|| path_text(&found))
        })
        .unwrap_or_default()
}

pub fn mise_installs_dir() -> PathBuf {
    if let Some(configured) = std::env::var_os("MISE_DATA_DIR").filter(|value| !value.is_empty()) {
        return PathBuf::from(configured).join("installs");
    }
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| expanded_path("~/.local/share"));
    base.join("mise").join("installs")
}

pub fn installed_agents() -> Value {
    let entries = AGENTS.iter().map(agent_entry).collect::<Vec<_>>();
    let installed = entries
        .iter()
        .filter(|entry| entry["installed"].as_bool().unwrap_or(false))
        .filter_map(|entry| entry["id"].as_str().map(ToOwned::to_owned))
        .collect::<Vec<_>>();
    json!({"ok": true, "agents": entries, "installed": installed})
}

fn first_mise_install(names: &[&str]) -> String {
    let root = mise_installs_dir();
    names
        .iter()
        .map(|name| root.join(name))
        .find(|candidate| {
            candidate.is_dir()
                && std::fs::read_dir(candidate)
                    .ok()
                    .and_then(|mut entries| entries.next())
                    .is_some()
        })
        .map(|path| path_text(&path))
        .unwrap_or_default()
}

fn first_home(paths: &[&str]) -> String {
    paths
        .iter()
        .map(|path| expanded_path(path))
        .find(|path| path.is_dir())
        .map(|path| path_text(&path))
        .unwrap_or_default()
}

fn agent_entry(agent: &Agent) -> Value {
    let binary = first_binary(agent.binaries);
    let mise = first_mise_install(agent.mise);
    json!({
        "id": agent.id,
        "label": agent.label,
        "installed": !binary.is_empty() || !mise.is_empty(),
        "binary": binary,
        "mise": mise,
        "home": first_home(agent.homes)
    })
}

use crate::actions::{plugin_root, read_manifest};
use crate::command::{CommandSpec, which};
use crate::common::path_text;
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use std::time::Duration;

const BLADE_SOCKET: &str = "data-goblin.fileblade/blade";
const HELPER_SOCKET: &str = "data-goblin.fileblade/helper";
const CORE_ID: &str = "data-goblin.fileblade";
const MAX_ENTRIES: usize = 256;
const MAX_PROVIDERS: usize = 128;
const MAX_MANIFEST_TOTAL_BYTES: usize = 4 * 1024 * 1024;
const MAX_DIAGNOSTICS: usize = 32;
const MAX_ROWS: usize = 512;
const CLI_TIMEOUT: Duration = Duration::from_secs(2);
const CLI_STDOUT_LIMIT: usize = 128 * 1024;
const CLI_STDERR_LIMIT: usize = 4 * 1024;

pub fn catalog() -> crate::AppResult<Value> {
    let mut diagnostics: Vec<Value> = Vec::new();
    let plugins = match plugins_dir() {
        Ok(plugins) => plugins,
        Err(error) => {
            return Ok(json!({
                "ok": true,
                "providers": [],
                "activation": "unknown",
                "truncated": false,
                "diagnostics": [json!({"source": "plugins", "error": bounded(&error, 256)})],
            }));
        }
    };
    let (activation, activation_state) = enabled_ids();
    let mut providers = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut ambiguous: HashSet<String> = HashSet::new();
    let mut bytes = 0usize;
    let mut truncated = false;
    let mut entries: Vec<PathBuf> = Vec::new();
    match std::fs::read_dir(&plugins) {
        Ok(reader) => {
            for entry in reader.take(MAX_ENTRIES + 1) {
                if entries.len() >= MAX_ENTRIES {
                    truncated = true;
                    break;
                }
                match entry {
                    Ok(entry) => entries.push(entry.path()),
                    Err(error) => {
                        note(&mut diagnostics, "plugins", &error.to_string());
                    }
                }
            }
        }
        Err(error) => {
            note(&mut diagnostics, &path_text(&plugins), &error.to_string());
        }
    }
    entries.sort();
    for path in entries {
        if providers.len() >= MAX_PROVIDERS || bytes >= MAX_MANIFEST_TOTAL_BYTES {
            truncated = true;
            break;
        }
        let name = match path.file_name().and_then(|value| value.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };
        if name.starts_with('.') || name == CORE_ID {
            continue;
        }
        if !path.is_dir() {
            continue;
        }
        let root = match plugin_root(&path.to_string_lossy()) {
            Ok(root) => root,
            Err(error) => {
                note(&mut diagnostics, &name, &error);
                continue;
            }
        };
        let manifest_path = root.join("manifest.json");
        match std::fs::symlink_metadata(&manifest_path) {
            Ok(metadata) if metadata.file_type().is_file() => {
                bytes = bytes.saturating_add(metadata.len() as usize);
            }
            Ok(_) => {
                note(
                    &mut diagnostics,
                    &name,
                    "manifest.json is not a regular file",
                );
                continue;
            }
            Err(_) => continue,
        }
        let manifest = match read_manifest(&root) {
            Ok(manifest) => manifest,
            Err(error) => {
                note(&mut diagnostics, &name, &error);
                continue;
            }
        };
        let id = manifest
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        if id.is_empty() || id != name || id == CORE_ID || id.starts_with("omarchy.") {
            if !id.is_empty() && id != name {
                note(
                    &mut diagnostics,
                    &name,
                    "the manifest id does not match its installed directory",
                );
            }
            continue;
        }
        if !contributes(&manifest) {
            continue;
        }
        if !seen.insert(id.clone()) {
            ambiguous.insert(id.clone());
            note(&mut diagnostics, &id, "two installed plugins carry this id");
            continue;
        }
        providers.push(json!({
            "id": id,
            "dir": path_text(&root),
            "path": path_text(&path),
            "manifest": manifest,
            "enabled": activation.get(&id).copied().unwrap_or(false),
        }));
    }
    let providers: Vec<Value> = providers
        .into_iter()
        .filter(|provider| {
            let id = provider
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or_default();
            !ambiguous.contains(id)
        })
        .collect();
    Ok(json!({
        "ok": true,
        "providers": providers,
        "activation": activation_state,
        "truncated": truncated,
        "diagnostics": diagnostics,
    }))
}

fn contributes(manifest: &Value) -> bool {
    let extensions = match manifest.get("extensions") {
        Some(Value::Object(extensions)) => extensions,
        _ => return false,
    };
    [BLADE_SOCKET, HELPER_SOCKET]
        .iter()
        .any(|socket| matches!(extensions.get(*socket), Some(Value::Array(entries)) if !entries.is_empty()))
}

fn plugins_dir() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME")
        .filter(|home| !home.is_empty())
        .ok_or("HOME is not set")?;
    let plugins = PathBuf::from(home).join(".config/omarchy/plugins");
    std::fs::canonicalize(&plugins).map_err(|error| format!("{}: {error}", path_text(&plugins)))
}

fn enabled_ids() -> (BTreeMap<String, bool>, &'static str) {
    let program = match which("omarchy") {
        Some(program) => program,
        None => return (BTreeMap::new(), "unknown"),
    };
    let output = CommandSpec::new(program)
        .args(["plugin", "list", "--json"])
        .env("LC_ALL", "C")
        .timeout(CLI_TIMEOUT)
        .limits(CLI_STDOUT_LIMIT, CLI_STDERR_LIMIT)
        .run();
    let output = match output {
        Ok(output) if output.status.success() && !output.stdout_truncated => output,
        _ => return (BTreeMap::new(), "unknown"),
    };
    let rows = match serde_json::from_slice::<Value>(&output.stdout) {
        Ok(Value::Array(rows)) => rows,
        _ => return (BTreeMap::new(), "unknown"),
    };
    if rows.len() > MAX_ROWS {
        return (BTreeMap::new(), "unknown");
    }
    let mut activation = BTreeMap::new();
    for row in rows {
        let id = row.get("id").and_then(Value::as_str).unwrap_or_default();
        let enabled = row.get("enabled").and_then(Value::as_bool);
        match (id.is_empty(), enabled) {
            (false, Some(enabled)) => {
                if activation.insert(id.to_string(), enabled).is_some() {
                    return (BTreeMap::new(), "unknown");
                }
            }
            _ => return (BTreeMap::new(), "unknown"),
        }
    }
    (activation, "known")
}

fn note(diagnostics: &mut Vec<Value>, source: &str, error: &str) {
    if diagnostics.len() >= MAX_DIAGNOSTICS {
        return;
    }
    diagnostics.push(json!({
        "source": bounded(source, 128),
        "error": bounded(error, 256),
    }));
}

fn bounded(value: &str, limit: usize) -> String {
    value
        .chars()
        .filter(|c| !c.is_control())
        .take(limit)
        .collect()
}

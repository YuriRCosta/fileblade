use crate::AppResult;
use crate::command::{CommandOutput, CommandSpec, which};
use crate::common::{parse_path, path_text};
use serde_json::{Value, json};
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

const MAX_REPOSITORIES: usize = 16;
const MAX_SUBJECTS: usize = 20;
const FETCH_TIMEOUT: Duration = Duration::from_secs(20);
const LOCAL_TIMEOUT: Duration = Duration::from_secs(5);
const OUTPUT_LIMIT: usize = 64 * 1024;

pub struct RepositorySpec {
    pub id: String,
    pub path: PathBuf,
}

pub fn parse_specs(raw: &[String]) -> Vec<RepositorySpec> {
    raw.iter()
        .take(MAX_REPOSITORIES)
        .filter_map(|entry| {
            let (id, path) = entry.split_once('=')?;
            let id = id.trim();
            if !(path.starts_with('/') || path.starts_with("file://")) {
                return None;
            }
            let path = parse_path(path).ok()?;
            let valid_id = !id.is_empty()
                && id.len() <= 128
                && id
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte));
            (valid_id && path.is_absolute()).then(|| RepositorySpec {
                id: id.to_string(),
                path,
            })
        })
        .collect()
}

fn git(
    path: &Path,
    arguments: &[&str],
    timeout: Duration,
    cancelled: &AtomicBool,
) -> AppResult<CommandOutput> {
    let program = which("git").ok_or_else(|| crate::AppError::command("git is not installed"))?;
    CommandSpec::new(program)
        .args(["-c", "core.fsmonitor=false", "-C"])
        .args([path])
        .args(arguments)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_ASKPASS", "")
        .env("GIT_SSH_COMMAND", "ssh -o BatchMode=yes")
        .env("LC_ALL", "C")
        .timeout(timeout)
        .limits(OUTPUT_LIMIT, 16 * 1024)
        .run_cancellable(cancelled)
}

fn git_text(path: &Path, arguments: &[&str], cancelled: &AtomicBool) -> Option<String> {
    let output = git(path, arguments, LOCAL_TIMEOUT, cancelled).ok()?;
    if !output.status.success() || output.stdout_truncated {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn manifest_version(text: &str) -> String {
    serde_json::from_str::<Value>(text)
        .ok()
        .and_then(|value| value["version"].as_str().map(str::to_string))
        .unwrap_or_default()
}

fn current_manifest_version(path: &Path) -> String {
    std::fs::read_to_string(path.join("manifest.json"))
        .map(|text| manifest_version(&text))
        .unwrap_or_default()
}

struct Standing {
    behind: u64,
    ahead: u64,
    dirty: bool,
    head: String,
    upstream: String,
    upstream_head: String,
}

fn standing(path: &Path, cancelled: &AtomicBool) -> Result<Standing, String> {
    let upstream = git_text(
        path,
        &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"],
        cancelled,
    )
    .filter(|value| !value.is_empty())
    .ok_or_else(|| "no upstream branch".to_string())?;
    let head = git_text(path, &["rev-parse", "HEAD"], cancelled).unwrap_or_default();
    let upstream_head = git_text(path, &["rev-parse", "@{u}"], cancelled).unwrap_or_default();
    let counts = git_text(
        path,
        &["rev-list", "--left-right", "--count", "HEAD...@{u}"],
        cancelled,
    )
    .ok_or_else(|| "unable to compare with upstream".to_string())?;
    let mut parts = counts.split_whitespace();
    let ahead = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    let behind = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    let status = git(
        path,
        &["status", "--porcelain=v1", "-z", "--untracked-files=no"],
        LOCAL_TIMEOUT,
        cancelled,
    )
    .map_err(|error| error.to_string())?;
    if !status.status.success() {
        return Err("unable to read the working tree status".to_string());
    }
    Ok(Standing {
        behind,
        ahead,
        dirty: !status.stdout.is_empty(),
        head,
        upstream,
        upstream_head,
    })
}

fn fetch_upstream(root: &Path, cancelled: &AtomicBool) -> Result<(), String> {
    let upstream = git_text(
        root,
        &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"],
        cancelled,
    )
    .filter(|value| !value.is_empty())
    .ok_or_else(|| "no upstream branch".to_string())?;
    let remote = upstream
        .split_once('/')
        .map(|(remote, _)| remote.to_string())
        .unwrap_or_else(|| "origin".to_string());
    match git(
        root,
        &["fetch", "--quiet", "--no-tags", &remote],
        FETCH_TIMEOUT,
        cancelled,
    ) {
        Ok(output) if output.status.success() => Ok(()),
        Ok(output) => {
            let text = String::from_utf8_lossy(&output.stderr);
            let line = text.lines().next().unwrap_or("fetch failed");
            Err(format!(
                "fetch failed: {}",
                line.chars().take(160).collect::<String>()
            ))
        }
        Err(error) => Err(format!("fetch failed: {error}")),
    }
}

fn repository_root(path: &Path, cancelled: &AtomicBool) -> Result<PathBuf, String> {
    let resolved = std::fs::canonicalize(path).map_err(|error| error.to_string())?;
    if !resolved.is_dir() {
        return Err("not a directory".to_string());
    }
    let output = git(
        &resolved,
        &["rev-parse", "--show-toplevel"],
        LOCAL_TIMEOUT,
        cancelled,
    )
    .map_err(|error| error.to_string())?;
    if !output.status.success() || output.stdout_truncated {
        return Err("not a git checkout".to_string());
    }
    let top = output.stdout.strip_suffix(b"\n").unwrap_or(&output.stdout);
    if std::fs::canonicalize(Path::new(OsStr::from_bytes(top)))
        .ok()
        .as_deref()
        != Some(resolved.as_path())
    {
        return Err("plugin directory is not the checkout root".to_string());
    }
    Ok(resolved)
}

fn failure(spec: &RepositorySpec, error: String) -> Value {
    json!({
        "id": spec.id,
        "path": path_text(&spec.path),
        "ok": false,
        "updatable": false,
        "error": error
    })
}

pub fn check(specs: &[RepositorySpec], core: &str, cancelled: &AtomicBool) -> Value {
    let mut repositories = Vec::with_capacity(specs.len());
    let mut available = false;
    for spec in specs {
        let root = match repository_root(&spec.path, cancelled) {
            Ok(root) => root,
            Err(error) => {
                repositories.push(failure(spec, error));
                continue;
            }
        };
        let fetch_error = match fetch_upstream(&root, cancelled) {
            Ok(()) => String::new(),
            Err(error) if error == "no upstream branch" => {
                repositories.push(failure(spec, error));
                continue;
            }
            Err(error) => error,
        };
        let standing = match standing(&root, cancelled) {
            Ok(standing) => standing,
            Err(error) => {
                repositories.push(failure(spec, error));
                continue;
            }
        };
        let subjects: Vec<String> = git_text(
            &root,
            &["log", "--format=%s", "--max-count=20", "HEAD..@{u}"],
            cancelled,
        )
        .map(|text| {
            text.lines()
                .take(MAX_SUBJECTS)
                .map(|line| line.chars().take(120).collect())
                .collect()
        })
        .unwrap_or_default();
        let current_version = current_manifest_version(&root);
        let upstream_version = git_text(&root, &["show", "@{u}:manifest.json"], cancelled)
            .map(|text| manifest_version(&text))
            .unwrap_or_default();
        let is_core = spec.id == core;
        let backend_stale = is_core && current_version != env!("CARGO_PKG_VERSION");
        let updatable =
            fetch_error.is_empty() && standing.behind > 0 && standing.ahead == 0 && !standing.dirty;
        available |= updatable;
        repositories.push(json!({
            "id": spec.id,
            "path": path_text(&root),
            "ok": fetch_error.is_empty(),
            "error": fetch_error,
            "behind": standing.behind,
            "ahead": standing.ahead,
            "dirty": standing.dirty,
            "head": standing.head,
            "upstream": standing.upstream,
            "upstream_head": standing.upstream_head,
            "current_version": current_version,
            "upstream_version": upstream_version,
            "subjects": subjects,
            "core": is_core,
            "backend_version": env!("CARGO_PKG_VERSION"),
            "backend_stale": backend_stale,
            "updatable": updatable
        }));
    }
    json!({
        "ok": true,
        "available": available,
        "checked_at": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|value| value.as_secs())
            .unwrap_or(0),
        "repositories": repositories
    })
}

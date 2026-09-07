use crate::command::{CommandSpec, which};
use serde_json::{Value, json};
use std::time::Duration;

pub struct Extension {
    pub url: &'static str,
    pub commit: &'static str,
}

pub const EXTENSIONS: &[Extension] = &[
    Extension {
        url: "https://github.com/data-goblin/fileblade-memory.git",
        commit: "1af97e1a34c71375aef1a8362b7e699ac13baa86",
    },
    Extension {
        url: "https://github.com/data-goblin/fileblade-skills.git",
        commit: "27f144884aca3e832cf298f6bf2abb459ca11635",
    },
    Extension {
        url: "https://github.com/data-goblin/fileblade-mcp.git",
        commit: "f71bbe5cffc21da5d60159d4842bc2726c740306",
    },
    Extension {
        url: "https://github.com/data-goblin/fileblade-hooks.git",
        commit: "05a64c9be745757924158b2b0ac27aab73859302",
    },
];
const INSTALL_TIMEOUT: Duration = Duration::from_secs(180);
const OUTPUT_LIMIT: usize = 64 * 1024;

pub fn allowed(url: &str) -> bool {
    pinned(url).is_some()
}

pub fn pinned(url: &str) -> Option<&'static Extension> {
    EXTENSIONS.iter().find(|extension| extension.url == url)
}

fn progress_path() -> std::path::PathBuf {
    crate::paths::state_dir().join("extension-install.json")
}

fn install_lock() -> std::io::Result<Option<crate::secure::LockedFile>> {
    crate::secure::try_open_private_lock(&crate::paths::state_dir().join("extension-install.lock"))
}

pub fn status() -> crate::AppResult<Value> {
    let lock = install_lock()?;
    let mut progress = match crate::secure::read_private_bounded(&progress_path(), OUTPUT_LIMIT)? {
        Some(data) => serde_json::from_slice(&data)?,
        None => json!({ "ok": true, "state": "idle", "installed": 0 }),
    };
    if lock.is_none() {
        progress["state"] = json!("running");
    } else if progress["state"] == "running" {
        progress["state"] = json!("failed");
        progress["message"] = json!("Installation was interrupted; click Install to retry");
    }
    Ok(progress)
}

pub fn install() -> crate::AppResult<Value> {
    let Some(_lock) = install_lock()? else {
        return status();
    };
    let mut progress = json!({
        "ok": true, "state": "running", "installed": 0,
        "total": EXTENSIONS.len(), "message": "",
    });
    let record = |progress: &Value| -> crate::AppResult<()> {
        crate::secure::write_private_atomic(&progress_path(), &serde_json::to_vec(progress)?)?;
        Ok(())
    };
    let fail = |mut progress: Value, message: String| -> crate::AppResult<Value> {
        progress["state"] = json!("failed");
        progress["message"] = json!(message);
        crate::secure::write_private_atomic(&progress_path(), &serde_json::to_vec(&progress)?)?;
        Ok(progress)
    };
    let plugins = match plugins_dir() {
        Ok(plugins) => plugins,
        Err(error) => return fail(progress, error),
    };
    let stage_root = stage_root(&plugins);
    let _ = std::fs::remove_dir_all(&stage_root);
    let mut staged = Vec::new();
    for (index, extension) in EXTENSIONS.iter().enumerate() {
        let url = extension.url;
        progress["url"] = json!(url);
        progress["commit"] = json!(extension.commit);
        progress["installed"] = json!(index);
        record(&progress)?;
        let id = extension_id(url);
        if plugins.join(&id).exists() {
            continue;
        }
        match stage_extension(extension, &id, &stage_root) {
            Ok(path) => staged.push((id, path)),
            Err(error) => {
                let _ = std::fs::remove_dir_all(&stage_root);
                return fail(progress, format!("Could not install {id}: {error}"));
            }
        }
    }
    let staged_ids: Vec<String> = staged.iter().map(|(id, _)| id.clone()).collect();
    for (id, path) in &staged {
        if let Err(error) = std::fs::rename(path, plugins.join(id)) {
            let _ = std::fs::remove_dir_all(&stage_root);
            return fail(progress, format!("Could not install {id}: {error}"));
        }
    }
    let _ = std::fs::remove_dir_all(&stage_root);
    progress["installed"] = json!(EXTENSIONS.len());
    record(&progress)?;
    if let Err(error) = enable_extensions(&plugins, &staged_ids) {
        return fail(progress, error);
    }
    progress["state"] = json!("installed");
    record(&progress)?;
    Ok(progress)
}

fn plugins_dir() -> Result<std::path::PathBuf, String> {
    let home = std::env::var_os("HOME")
        .filter(|home| !home.is_empty())
        .ok_or("HOME is not set")?;
    let plugins = std::path::PathBuf::from(home).join(".config/omarchy/plugins");
    std::fs::create_dir_all(&plugins).map_err(|error| error.to_string())?;
    std::fs::canonicalize(&plugins).map_err(|error| error.to_string())
}

fn stage_root(plugins: &std::path::Path) -> std::path::PathBuf {
    plugins
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| plugins.to_path_buf())
        .join(".fileblade-extension-stage")
}

fn git_in(stage: &std::path::Path, arguments: &[&str]) -> Result<String, String> {
    let git = which("git").ok_or("git is not on PATH")?;
    let result = CommandSpec::new(git)
        .args(arguments)
        .env("GIT_TERMINAL_PROMPT", "0")
        .cwd(stage)
        .timeout(INSTALL_TIMEOUT)
        .limits(OUTPUT_LIMIT, OUTPUT_LIMIT)
        .retain_tail(true)
        .run()
        .map_err(|error| error.to_string())?;
    if !result.status.success() {
        return Err(String::from_utf8_lossy(&result.stderr)
            .lines()
            .last()
            .unwrap_or("git command failed")
            .to_string());
    }
    Ok(String::from_utf8_lossy(&result.stdout).trim().to_string())
}

fn stage_extension(
    extension: &Extension,
    id: &str,
    stage_root: &std::path::Path,
) -> Result<std::path::PathBuf, String> {
    let url = extension.url;
    let commit = extension.commit;
    omarchy_command("omarchy-git-url-check", &[url])?;
    std::fs::create_dir_all(stage_root).map_err(|error| error.to_string())?;
    let stage = stage_root.join(id);
    let _ = std::fs::remove_dir_all(&stage);
    let git = which("git").ok_or("git is not on PATH")?;
    let clone = CommandSpec::new(git)
        .args(["clone", "--", url, &stage.to_string_lossy()])
        .env("GIT_TERMINAL_PROMPT", "0")
        .timeout(INSTALL_TIMEOUT)
        .limits(OUTPUT_LIMIT, OUTPUT_LIMIT)
        .retain_tail(true)
        .run()
        .map_err(|error| error.to_string())?;
    if !clone.status.success() {
        return Err(String::from_utf8_lossy(&clone.stderr)
            .lines()
            .last()
            .unwrap_or("git clone failed")
            .to_string());
    }
    git_in(
        &stage,
        &["rev-parse", "--verify", &format!("{commit}^{{commit}}")],
    )
    .map_err(|_| format!("{url} does not contain the reviewed commit {commit}"))?;
    git_in(&stage, &["reset", "--hard", commit])
        .map_err(|error| format!("could not check out {commit}: {error}"))?;
    let head = git_in(&stage, &["rev-parse", "HEAD"])?;
    if head != commit {
        return Err(format!(
            "{url} is at {head} instead of the reviewed commit {commit}"
        ));
    }
    omarchy_command("omarchy-plugin-validate", &[&stage.to_string_lossy()])?;
    let stage = std::fs::canonicalize(&stage).map_err(|error| error.to_string())?;
    let manifest = crate::secure::read_bounded_nofollow(&stage.join("manifest.json"), OUTPUT_LIMIT)
        .map_err(|error| error.to_string())?
        .ok_or("cloned manifest is missing")?;
    let manifest: Value = serde_json::from_slice(&manifest).map_err(|error| error.to_string())?;
    if manifest["id"] != id {
        return Err("cloned plugin has a different ID".to_string());
    }
    Ok(stage)
}

fn enabled_ids() -> Result<std::collections::HashMap<String, bool>, String> {
    let plugins = omarchy_command("omarchy-plugin-list", &["--json"])?;
    let Value::Array(plugins) =
        serde_json::from_str(&plugins).map_err(|error| error.to_string())?
    else {
        return Err("omarchy-plugin-list returned no list".to_string());
    };
    Ok(plugins
        .iter()
        .filter_map(|plugin| {
            Some((
                plugin["id"].as_str()?.to_string(),
                plugin["enabled"] == true,
            ))
        })
        .collect())
}

fn enable_extensions(plugins: &std::path::Path, staged: &[String]) -> Result<(), String> {
    let ids: Vec<String> = EXTENSIONS
        .iter()
        .map(|extension| extension_id(extension.url))
        .collect();
    for id in &ids {
        let target = std::fs::canonicalize(plugins.join(id)).map_err(|error| error.to_string())?;
        if !staged.contains(id) {
            omarchy_command("omarchy-plugin-validate", &[&target.to_string_lossy()])?;
        }
        let manifest =
            crate::secure::read_bounded_nofollow(&target.join("manifest.json"), OUTPUT_LIMIT)
                .map_err(|error| error.to_string())?
                .ok_or("installed manifest is missing")?;
        let manifest: Value =
            serde_json::from_slice(&manifest).map_err(|error| error.to_string())?;
        if manifest["id"] != *id {
            return Err(format!("installed plugin {id} has a different ID"));
        }
    }
    let deadline = std::time::Instant::now() + Duration::from_secs(60);
    let mut error = String::from("shell did not enable the extensions");
    while std::time::Instant::now() < deadline {
        let known = enabled_ids().unwrap_or_default();
        if ids.iter().all(|id| known.get(id) == Some(&true)) {
            return Ok(());
        }
        if ids.iter().all(|id| known.contains_key(id)) {
            for id in ids.iter().filter(|id| known.get(*id) != Some(&true)) {
                if let Err(message) = omarchy_command("omarchy-plugin-enable", &[id]) {
                    error = message;
                }
            }
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    Err(error)
}

fn extension_id(url: &str) -> String {
    format!(
        "data-goblin.{}",
        url.rsplit('/')
            .next()
            .unwrap_or("")
            .trim_end_matches(".git")
    )
}

fn omarchy_command(name: &str, arguments: &[&str]) -> Result<String, String> {
    let program = which(name).ok_or_else(|| format!("{name} is not on PATH"))?;
    let result = CommandSpec::new(program)
        .args(arguments)
        .timeout(Duration::from_secs(5))
        .limits(OUTPUT_LIMIT, OUTPUT_LIMIT)
        .retain_tail(true)
        .run()
        .map_err(|error| error.to_string())?;
    if result.status.success() {
        Ok(String::from_utf8_lossy(&result.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&result.stderr)
            .lines()
            .last()
            .unwrap_or("command failed")
            .to_string())
    }
}

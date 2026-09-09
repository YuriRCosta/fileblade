use crate::command::{CommandSpec, which};
use serde_json::{Value, json};
use std::time::Duration;

mod staging;

pub struct Extension {
    pub url: &'static str,
    pub commit: &'static str,
}

pub const EXTENSIONS: &[Extension] = &[
    Extension {
        url: "https://github.com/data-goblin/fileblade-memory.git",
        commit: "3a3fd33263b12d15c6d7982f1503e6daedc265ce",
    },
    Extension {
        url: "https://github.com/data-goblin/fileblade-skills.git",
        commit: "742b5f3e142f8eacba9aba766bcc67d3f86e7be2",
    },
    Extension {
        url: "https://github.com/data-goblin/fileblade-mcp.git",
        commit: "5dc90ae98eeaf1f10185941d344f317540242441",
    },
    Extension {
        url: "https://github.com/data-goblin/fileblade-hooks.git",
        commit: "5b91b08b6f6c47e22284031ccaa6ad541d0ba54d",
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
    let stage_root = match staging::Staging::create(&plugins) {
        Ok(stage) => stage,
        Err(error) => return fail(progress, error.to_string()),
    };
    let mut staged = Vec::new();
    for (index, extension) in EXTENSIONS.iter().enumerate() {
        let url = extension.url;
        progress["url"] = json!(url);
        progress["commit"] = json!(extension.commit);
        progress["installed"] = json!(index);
        record(&progress)?;
        let id = extension_id(url);
        let target = plugins.join(&id);
        if crate::secure::entry_exists(&target)? {
            if let Err(error) = verify_existing(extension, &id, &target) {
                return fail(
                    progress,
                    format!(
                        "Existing {id} was preserved: {error}. Update or enable it explicitly."
                    ),
                );
            }
            continue;
        }
        match stage_extension(extension, &id, &stage_root.path) {
            Ok(path) => staged.push((id, path)),
            Err(error) => {
                return fail(progress, format!("Could not install {id}: {error}"));
            }
        }
    }
    let staged_ids: Vec<String> = staged.iter().map(|(id, _)| id.clone()).collect();
    for (id, path) in &staged {
        if let Err(error) = crate::secure::rename_noreplace(path, &plugins.join(id)) {
            return fail(progress, format!("Could not install {id}: {error}"));
        }
    }
    stage_root.clear()?;
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

fn git_in(stage: &std::path::Path, arguments: &[&str]) -> Result<String, String> {
    let git = which("git").ok_or("git is not on PATH")?;
    let mut command = CommandSpec::new(git)
        .args([
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "core.fsmonitor=false",
            "-c",
            "transfer.unpackLimit=0",
            "-c",
            "fetch.unpackLimit=0",
            "-c",
            "gc.auto=0",
            "-c",
            "maintenance.auto=false",
            "-c",
            "http.followRedirects=false",
            "-c",
            "protocol.file.allow=never",
            "-c",
            "protocol.ext.allow=never",
        ])
        .args(arguments)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_ASKPASS", "")
        .env("GIT_LFS_SKIP_SMUDGE", "1")
        .resource_limits(16 * 1024 * 1024, 512 * 1024 * 1024)
        .stop_on_output_limit()
        .cwd(stage)
        .timeout(INSTALL_TIMEOUT)
        .limits(OUTPUT_LIMIT, OUTPUT_LIMIT);
    if let Some(root) = stage.ancestors().find(|path| {
        path.file_name()
            .is_some_and(|name| name.as_encoded_bytes().starts_with(b".fileblade-partial-"))
    }) {
        command = command.directory_budget(root, 128 * 1024 * 1024, 16384);
    }
    let result = command.run().map_err(|error| error.to_string())?;
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
    let stage = stage_root.join(id);
    git_in(
        stage_root,
        &[
            "clone",
            "--no-checkout",
            "--depth=1",
            "--no-tags",
            "--template=",
            "--revision",
            commit,
            "--",
            url,
            &stage.to_string_lossy(),
        ],
    )
    .map_err(|error| format!("could not acquire reviewed commit {commit}: {error}"))?;
    git_in(
        &stage,
        &["rev-parse", "--verify", &format!("{commit}^{{commit}}")],
    )
    .map_err(|_| format!("{url} does not contain the reviewed commit {commit}"))?;
    check_tree(&stage, commit)?;
    git_in(&stage, &["checkout", "--detach", "--force", commit])
        .map_err(|error| format!("could not check out {commit}: {error}"))?;
    let head = git_in(&stage, &["rev-parse", "HEAD"])?;
    if head != commit {
        return Err(format!(
            "{url} is at {head} instead of the reviewed commit {commit}"
        ));
    }
    verify_existing(extension, id, &stage)?;
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

fn check_tree(stage: &std::path::Path, commit: &str) -> Result<(), String> {
    let tree = git_in(stage, &["ls-tree", "-r", "-l", "-z", commit])?;
    let mut count = 0;
    let mut bytes = 0_u64;
    for entry in tree.split('\0').filter(|entry| !entry.is_empty()) {
        count += 1;
        let (header, name) = entry.split_once('\t').ok_or("invalid pinned tree")?;
        let fields: Vec<_> = header.split_whitespace().collect();
        if count > 2048
            || fields.len() != 4
            || !matches!(fields[0], "100644" | "100755")
            || fields[1] != "blob"
            || name.len() > 1024
            || name.split('/').count() > 16
        {
            return Err("pinned tree exceeds the file, path or file-type limit".into());
        }
        let size = fields[3]
            .parse::<u64>()
            .map_err(|_| "invalid pinned file size")?;
        bytes = bytes.checked_add(size).ok_or("pinned tree is too large")?;
        if bytes > 8 * 1024 * 1024 {
            return Err("pinned tree exceeds 8 MiB".into());
        }
    }
    Ok(())
}

fn verify_existing(
    extension: &Extension,
    id: &str,
    target: &std::path::Path,
) -> Result<(), String> {
    let root = git_in(target, &["rev-parse", "--show-toplevel"])?;
    if std::fs::canonicalize(root).ok() != std::fs::canonicalize(target).ok() {
        return Err("plugin directory is not the checkout root".into());
    }
    if git_in(target, &["remote", "get-url", "origin"])? != extension.url {
        return Err("checkout has a different origin".into());
    }
    if git_in(target, &["rev-parse", "HEAD"])? != extension.commit {
        return Err("checkout is not at the reviewed commit".into());
    }
    if !git_in(
        target,
        &[
            "status",
            "--porcelain=v1",
            "--untracked-files=all",
            "--ignored",
        ],
    )?
    .is_empty()
    {
        return Err("checkout has local or untracked changes".into());
    }
    let manifest =
        crate::secure::read_bounded_nofollow(&target.join("manifest.json"), OUTPUT_LIMIT)
            .map_err(|error| error.to_string())?
            .ok_or("manifest is missing")?;
    let manifest: Value = serde_json::from_slice(&manifest).map_err(|error| error.to_string())?;
    if manifest["id"] != id {
        return Err("installed plugin has a different ID".into());
    }
    omarchy_command("omarchy-plugin-validate", &[&target.to_string_lossy()])?;
    Ok(())
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

fn enable_extensions(plugins: &std::path::Path, _staged: &[String]) -> Result<(), String> {
    let ids: Vec<String> = EXTENSIONS
        .iter()
        .map(|extension| extension_id(extension.url))
        .collect();
    for id in &ids {
        let target = std::fs::canonicalize(plugins.join(id)).map_err(|error| error.to_string())?;
        let extension = EXTENSIONS
            .iter()
            .find(|extension| extension_id(extension.url) == *id)
            .unwrap();
        verify_existing(extension, id, &target)?;
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

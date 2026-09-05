use super::*;

const SELECTION_PREFIX: &str = "selection-";
const SELECTION_RETENTION: Duration = Duration::from_secs(3600);
const MAX_PRUNED_FILES: usize = 512;

pub(super) fn selection_document(targets: &[PathBuf]) -> Value {
    Value::Array(
        targets
            .iter()
            .map(|path| {
                let name = crate::common::file_name(path);
                let symlink = std::fs::symlink_metadata(path)
                    .map(|metadata| metadata.file_type().is_symlink())
                    .unwrap_or(false);
                let resolved = std::fs::metadata(path).ok();
                let is_directory = resolved
                    .as_ref()
                    .map(std::fs::Metadata::is_dir)
                    .unwrap_or(false);
                json!({
                    "path": path_text(path),
                    "name": name,
                    "dir": is_directory,
                    "symlink": symlink,
                    "size": resolved.as_ref().map(std::fs::Metadata::len),
                    "mime": entry_mime(&name, is_directory),
                })
            })
            .collect(),
    )
}

pub(super) fn write_selection_file(state_dir: &Path, selection: &str) -> AppResult<PathBuf> {
    let path = state_dir.join(format!(
        "{SELECTION_PREFIX}{}.json",
        Uuid::new_v4().simple()
    ));
    write_new_private(&path, selection.as_bytes())?;
    Ok(path)
}

pub(super) fn remove_selection_file(path: Option<&Path>) {
    if let Some(path) = path {
        let _ = remove_path(path);
    }
}

pub(super) fn prune_selection_files(state_dir: &Path) {
    let Some(cutoff) = SystemTime::now().checked_sub(SELECTION_RETENTION) else {
        return;
    };
    let Ok(entries) = std::fs::read_dir(state_dir) else {
        return;
    };
    for entry in entries.flatten().take(MAX_PRUNED_FILES) {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if !name.starts_with(SELECTION_PREFIX) || !name.ends_with(".json") {
            continue;
        }
        let stale = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .map(|modified| modified < cutoff)
            .unwrap_or(false);
        if stale {
            let _ = remove_path(&entry.path());
        }
    }
}

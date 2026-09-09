use crate::{AppError, AppResult};
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub(super) struct Budget {
    pub path: PathBuf,
    pub bytes: u64,
    pub entries: usize,
}

impl Budget {
    pub fn check(&self) -> AppResult<()> {
        let mut pending = vec![(self.path.clone(), 0)];
        let mut bytes = 0_u64;
        let mut entries = 0;
        while let Some((directory, depth)) = pending.pop() {
            if depth > 32 {
                return Err(AppError::command(
                    "command staging exceeds 32 directory levels",
                ));
            }
            let names = match std::fs::read_dir(&directory) {
                Ok(names) => names,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error.into()),
            };
            for entry in names {
                let entry = entry?;
                entries += 1;
                if entries > self.entries {
                    return Err(AppError::command("command staging exceeds its entry limit"));
                }
                let metadata = match std::fs::symlink_metadata(entry.path()) {
                    Ok(metadata) => metadata,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                    Err(error) => return Err(error.into()),
                };
                if metadata.is_dir() {
                    pending.push((entry.path(), depth + 1));
                } else if metadata.is_file() {
                    bytes = bytes.saturating_add(metadata.len());
                    if bytes > self.bytes {
                        return Err(AppError::command("command staging exceeds its byte limit"));
                    }
                } else {
                    return Err(AppError::command(
                        "command staging contains a non-regular entry",
                    ));
                }
            }
        }
        Ok(())
    }
}

pub(super) fn budget(path: &Path, bytes: u64, entries: usize) -> Budget {
    Budget {
        path: path.to_path_buf(),
        bytes,
        entries,
    }
}

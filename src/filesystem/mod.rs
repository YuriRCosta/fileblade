use crate::command::{CommandSpec, which};
use crate::common::{
    display_path, error_payload, file_name, human_size, parse_path, path_error, path_text,
    timestamp, timestamp_seconds,
};
use crate::git::{
    GitRepository, cached_git_worktree_status, decorate_git_entry, deleted_git_entry,
    git_metadata_document, ignored_paths, indexed_git_counts_for_path, indexed_git_status_for_path,
    nearest_git_marker, repository_status_index_cancellable,
};
use rustix::fs::{Access, AtFlags, CWD, Mode, OFlags, StatxFlags, accessat, open, statx};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File, Metadata};
use std::io::{self, Read};
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, UNIX_EPOCH};

mod content;
mod entries;
mod listing;
pub use content::*;
pub use entries::*;
pub use listing::*;
pub const DIRECTORY_ENTRY_LIMIT: usize = 1000;
pub const DIRECTORY_SCAN_LIMIT: usize = 10_000;
pub const MAX_TEXT_BYTES: usize = 1024 * 1024;
const IDENTITY_DATABASE_LIMIT: usize = 1024 * 1024;
const MIME_DATABASE_LIMIT: usize = 512 * 1024;

pub fn has_git_marker(directory: &Path) -> bool {
    let marker = directory.join(".git");
    if marker.is_dir() {
        return regular_nofollow(&marker.join("HEAD"));
    }
    read_regular_prefix(&marker, 256)
        .map(|data| trim_ascii_start(&data).starts_with(b"gitdir:"))
        .unwrap_or(false)
}

pub(crate) fn read_regular_file(path: &Path, limit: usize) -> io::Result<Vec<u8>> {
    let mut file = open_regular(path)?;
    let metadata = file.metadata()?;
    if metadata.len() > limit as u64 {
        return Err(io::Error::new(
            io::ErrorKind::FileTooLarge,
            "bounded read exceeded",
        ));
    }
    let mut data = Vec::with_capacity((metadata.len() as usize).min(limit));
    file.by_ref()
        .take(limit as u64 + 1)
        .read_to_end(&mut data)?;
    if data.len() > limit {
        return Err(io::Error::new(
            io::ErrorKind::FileTooLarge,
            "bounded read exceeded",
        ));
    }
    Ok(data)
}

pub(crate) fn read_regular_prefix(path: &Path, limit: usize) -> io::Result<Vec<u8>> {
    let mut file = open_regular(path)?;
    let mut data = Vec::with_capacity(limit.min(64 * 1024));
    file.by_ref().take(limit as u64).read_to_end(&mut data)?;
    Ok(data)
}

pub(crate) fn entry_mime(name: &str, is_dir: bool) -> String {
    if is_dir {
        return "inode/directory".to_string();
    }
    let lower = name.to_lowercase();
    let database = mime_database();
    lower
        .match_indices('.')
        .map(|(index, _)| &lower[index + 1..])
        .find_map(|extension| database.get(extension).map(|mime| mime.to_string()))
        .unwrap_or_else(|| "application/octet-stream".to_string())
}

pub(crate) fn valid_mime(value: &str) -> bool {
    let Some((media_type, subtype)) = value.split_once('/') else {
        return false;
    };
    if value.len() > 255
        || value.starts_with('-')
        || media_type.is_empty()
        || subtype.is_empty()
        || subtype.contains('/')
    {
        return false;
    }
    value.bytes().all(|byte| {
        byte.is_ascii_alphanumeric()
            || matches!(
                byte,
                b'/' | b'!' | b'#' | b'$' | b'&' | b'^' | b'_' | b'.' | b'+' | b'-'
            )
    })
}

pub(crate) fn basic_entry_with_git(
    path: &Path,
    metadata: &Metadata,
    is_dir: bool,
    is_link: bool,
    git_enabled: bool,
) -> Value {
    let name = file_name(path);
    let size = (!is_dir).then_some(metadata.len());
    json!({
        "name": name,
        "path": path_text(path),
        "is_dir": is_dir,
        "is_symlink": is_link,
        "is_git_repo": git_enabled && is_dir && marker_from_metadata(path),
        "is_deleted": false,
        "size": size.map(|value| value as i64).unwrap_or(-1),
        "size_text": human_size(size),
        "modified": metadata.modified().map(timestamp).unwrap_or_default(),
        "created": "",
        "stat_fingerprint": format!(
            "{}:{}:{}:{}:{}:{}",
            metadata.mode(),
            metadata.size(),
            metadata.mtime() * 1_000_000_000 + metadata.mtime_nsec(),
            metadata.ctime() * 1_000_000_000 + metadata.ctime_nsec(),
            metadata.uid(),
            metadata.gid()
        ),
        "kind": entry_kind(metadata, is_dir, is_link),
        "mime": entry_mime(&name, is_dir),
        "git_repo_root": "",
        "git_status": "",
        "git_status_label": "",
        "git_index_status": "",
        "git_worktree_status": "",
        "git_original_path": "",
        "git_modified_count": 0,
        "git_deleted_count": 0,
        "git_new_count": 0
    })
}

pub(crate) fn cached_git_repository(
    path: &Path,
    cache: &mut HashMap<PathBuf, GitRepository>,
    refresh: bool,
    cancelled: &AtomicBool,
) -> Option<GitRepository> {
    let cached_root = cache
        .keys()
        .filter(|root| path == root.as_path() || path.starts_with(root))
        .max_by_key(|root| root.as_os_str().len())
        .cloned();
    if let Some(root) = cached_root {
        let mut cursor = path.to_path_buf();
        while cursor != root {
            if has_git_marker(&cursor) {
                let repository = cached_git_worktree_status(
                    &cursor,
                    refresh,
                    std::time::Duration::from_secs(4),
                    cancelled,
                )?;
                cache.insert(repository.root.clone(), repository.clone());
                return Some(repository);
            }
            if !cursor.pop() {
                break;
            }
        }
        return cache.get(&root).cloned();
    }
    let marker = nearest_git_marker(&path_text(path), &mut HashMap::new())?;
    let repository = cached_git_worktree_status(
        &marker,
        refresh,
        std::time::Duration::from_secs(4),
        cancelled,
    )?;
    cache.insert(repository.root.clone(), repository.clone());
    Some(repository)
}

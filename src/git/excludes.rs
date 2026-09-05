use crate::command::{CommandSpec, which};
use std::collections::HashSet;
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

const MAX_EXCLUDE_PATHS: usize = 4096;
const MAX_EXCLUDE_STDOUT_BYTES: usize = 4 * 1024 * 1024;

pub fn ignored_paths(root: &Path, paths: &[PathBuf], cancelled: &AtomicBool) -> HashSet<PathBuf> {
    let mut ignored = HashSet::new();
    let mut payload = Vec::new();
    let mut queried = 0_usize;
    for path in paths {
        if queried >= MAX_EXCLUDE_PATHS {
            break;
        }
        if path != root && path.starts_with(root) {
            payload.extend_from_slice(path.as_os_str().as_bytes());
            payload.push(0);
            queried += 1;
        }
    }
    if payload.is_empty() {
        return ignored;
    }
    let Some(git) = which("git") else {
        return ignored;
    };
    let result = CommandSpec::new(git)
        .args(["-c", "core.fsmonitor=false", "-C"])
        .args([root])
        .args(["check-ignore", "-z", "--stdin"])
        .env("GIT_OPTIONAL_LOCKS", "0")
        .stdin(payload)
        .timeout(Duration::from_secs(4))
        .limits(MAX_EXCLUDE_STDOUT_BYTES, 64 * 1024)
        .run_cancellable(cancelled);
    let Ok(output) = result else {
        return ignored;
    };
    if output.stdout_truncated || !matches!(output.status.code(), Some(0) | Some(1)) {
        return ignored;
    }
    for record in output.stdout.split(|byte| *byte == 0) {
        if !record.is_empty() {
            ignored.insert(PathBuf::from(OsStr::from_bytes(record)));
        }
    }
    ignored
}

use super::*;
use std::hash::{DefaultHasher, Hash, Hasher};

pub fn fingerprint(path: &Path) -> Option<Fingerprint> {
    let stat = secure::entry_stat(path).ok()?;
    let kind = match stat.kind {
        EntryKind::Symlink => "link",
        EntryKind::Directory => "dir",
        EntryKind::File => "file",
        EntryKind::Other => "file",
    };
    Some(Fingerprint {
        dev: stat.dev,
        ino: stat.ino,
        size: stat.size,
        mtime_ns: nanoseconds(stat.mtime, stat.mtime_nsec),
        ctime_ns: Some(nanoseconds(stat.ctime, stat.ctime_nsec)),
        kind: kind.to_string(),
        tree: (kind == "dir").then(|| tree_digest(path)),
    })
}

pub(super) fn tree_digest(path: &Path) -> TreeFingerprint {
    let mut count = 0_u64;
    let mut bytes = 0_u64;
    let mut newest = 0_i64;
    let mut truncated = false;
    let mut digest = DefaultHasher::new();
    let mut pending = VecDeque::from([path.to_path_buf()]);
    while let Some(current) = pending.pop_back() {
        let mut names = match secure::directory_names(&current) {
            Ok(names) => names,
            Err(_) => {
                truncated = true;
                break;
            }
        };
        names.sort();
        for name in names {
            count += 1;
            if count > TREE_DIGEST_LIMIT as u64 {
                truncated = true;
                break;
            }
            let child = current.join(name);
            let stat = match secure::entry_stat(&child) {
                Ok(stat) => stat,
                Err(_) => {
                    truncated = true;
                    break;
                }
            };
            child.strip_prefix(path).ok().hash(&mut digest);
            (
                stat.dev,
                stat.ino,
                stat.mode,
                stat.uid,
                stat.size,
                stat.mtime,
                stat.mtime_nsec,
                stat.ctime,
                stat.ctime_nsec,
            )
                .hash(&mut digest);
            newest = newest.max(nanoseconds(stat.mtime, stat.mtime_nsec));
            if stat.kind == EntryKind::Directory {
                pending.push_back(child);
            } else {
                bytes = bytes.saturating_add(stat.size);
            }
        }
        if truncated {
            break;
        }
    }
    TreeFingerprint {
        digest: format!("{:016x}", digest.finish()),
        count,
        bytes,
        mtime_ns: newest,
        truncated,
    }
}

pub(super) fn nanoseconds(seconds: i64, nanoseconds: i64) -> i64 {
    seconds
        .saturating_mul(1_000_000_000)
        .saturating_add(nanoseconds)
}

pub(super) fn same_identity(recorded: Option<&Fingerprint>, path: &Path) -> bool {
    let Some(recorded) = recorded else {
        return false;
    };
    secure::entry_stat(path)
        .map(|stat| stat.dev == recorded.dev && stat.ino == recorded.ino)
        .unwrap_or(false)
}

pub(super) fn unchanged(recorded: Option<&Fingerprint>, path: &Path) -> (bool, bool) {
    let Some(recorded) = recorded else {
        return (false, false);
    };
    if !same_identity(Some(recorded), path) {
        return (false, false);
    }
    let Some(current) = fingerprint(path) else {
        return (false, false);
    };
    if current.kind != recorded.kind {
        return (false, false);
    }
    if current.kind == "dir" {
        let Some(recorded_tree) = recorded.tree.as_ref() else {
            return (false, false);
        };
        let Some(current_tree) = current.tree.as_ref() else {
            return (false, false);
        };
        if recorded_tree.truncated || current_tree.truncated || recorded_tree.digest.is_empty() {
            return (false, true);
        }
        return (
            recorded_tree.digest == current_tree.digest
                && recorded_tree.count == current_tree.count
                && recorded_tree.bytes == current_tree.bytes
                && recorded_tree.mtime_ns == current_tree.mtime_ns,
            false,
        );
    }
    (
        current.size == recorded.size
            && current.mtime_ns == recorded.mtime_ns
            && recorded.ctime_ns.is_some()
            && current.ctime_ns == recorded.ctime_ns,
        false,
    )
}

pub(super) fn same_fingerprint_identity(
    expected: &Fingerprint,
    current: secure::EntryStat,
) -> bool {
    fingerprint_identity(expected).is_some_and(|value| value == current.identity())
}

pub(super) fn fingerprint_identity(value: &Fingerprint) -> Option<secure::EntryIdentity> {
    let kind = match value.kind.as_str() {
        "link" => EntryKind::Symlink,
        "dir" => EntryKind::Directory,
        "file" => EntryKind::File,
        _ => return None,
    };
    Some(secure::EntryIdentity {
        dev: value.dev,
        ino: value.ino,
        kind,
    })
}

use super::*;

pub fn entry_for(raw_path: &str, include_details: bool) -> io::Result<Value> {
    entry_for_path(&parse_path(raw_path)?, include_details)
}

pub fn entry_for_path(path: &Path, include_details: bool) -> io::Result<Value> {
    entry_for_path_with_git(path, include_details, true)
}

pub(crate) fn entry_for_path_with_git(
    path: &Path,
    include_details: bool,
    git_enabled: bool,
) -> io::Result<Value> {
    entry_for_path_cancellable_with_git(path, include_details, git_enabled, &AtomicBool::new(false))
}

pub fn entry_for_path_cancellable(
    path: &Path,
    include_details: bool,
    cancelled: &AtomicBool,
) -> io::Result<Value> {
    entry_for_path_cancellable_with_git(path, include_details, true, cancelled)
}

fn entry_for_path_cancellable_with_git(
    path: &Path,
    include_details: bool,
    git_enabled: bool,
    cancelled: &AtomicBool,
) -> io::Result<Value> {
    if cancelled.load(Ordering::Relaxed) {
        return Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "operation cancelled",
        ));
    }
    let metadata = fs::symlink_metadata(path)?;
    let is_link = metadata.file_type().is_symlink();
    let is_dir = path.is_dir();
    let mut item = basic_entry_with_git(path, &metadata, is_dir, is_link, git_enabled);
    if include_details {
        item["created"] = json!(creation_timestamp(path));
        let mime = content_type_path_cancellable(path, cancelled);
        if cancelled.load(Ordering::Relaxed) {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "operation cancelled",
            ));
        }
        item["mime"] = json!(mime);
        let details = detailed_entry(path, &metadata, is_link);
        if let (Some(target), Some(source)) = (item.as_object_mut(), details.as_object()) {
            target.extend(source.clone());
        }
    }
    Ok(item)
}

pub fn stat_path(raw_path: &str) -> Value {
    stat_path_cancellable(raw_path, &AtomicBool::new(false))
}

pub fn stat_path_cancellable(raw_path: &str, cancelled: &AtomicBool) -> Value {
    let path = match parse_path(raw_path) {
        Ok(path) => path,
        Err(error) => return path_error(raw_path, &error),
    };
    match entry_for_path_cancellable(&path, true, cancelled) {
        Ok(entry) => json!({"ok": true, "entry": entry}),
        Err(error) => error_payload(&path, &error),
    }
}

pub fn stat_paths(paths: &[String]) -> Value {
    let mut entries = Vec::new();
    let mut errors = Vec::new();
    let mut seen = HashSet::new();
    for raw_path in paths.iter().take(512) {
        let path = match parse_path(raw_path) {
            Ok(path) => path,
            Err(error) => {
                errors.push(path_error(raw_path, &error));
                continue;
            }
        };
        if !seen.insert(path.clone()) {
            continue;
        }
        match entry_for_path(&path, false) {
            Ok(entry) => entries.push(entry),
            Err(error) => errors.push(error_payload(&path, &error)),
        }
    }
    let message = errors
        .iter()
        .filter_map(|item| {
            Some(format!(
                "{}: {}",
                item.get("path")?.as_str()?,
                item.get("error")?.as_str()?
            ))
        })
        .collect::<Vec<_>>()
        .join("; ");
    json!({
        "ok": errors.is_empty(),
        "entries": entries,
        "errors": errors,
        "error": message
    })
}

pub(super) fn detailed_entry(path: &Path, metadata: &Metadata, is_link: bool) -> Value {
    let mode = metadata.mode();
    json!({
        "permissions": permissions(mode),
        "mode": format!("{:04o}", mode & 0o7777),
        "owner": owner_name(metadata.uid()),
        "group": group_name(metadata.gid()),
        "accessed": metadata.accessed().map(timestamp).unwrap_or_default(),
        "changed": timestamp_seconds(metadata.ctime()),
        "parent": path_text(path.parent().unwrap_or_else(|| Path::new("/"))),
        "link_target": if is_link { fs::read_link(path).map(|value| display_path(&value)).unwrap_or_default() } else { String::new() },
        "executable": mode & 0o111 != 0
    })
}

pub(super) fn entry_kind(metadata: &Metadata, is_dir: bool, is_link: bool) -> &'static str {
    if is_link {
        return if is_dir {
            "Symlink to directory"
        } else {
            "Symbolic link"
        };
    }
    if is_dir {
        return "Directory";
    }
    let kind = metadata.file_type();
    if kind.is_file() {
        "File"
    } else if kind.is_socket() {
        "Socket"
    } else if kind.is_fifo() {
        "Named pipe"
    } else if kind.is_char_device() {
        "Character device"
    } else if kind.is_block_device() {
        "Block device"
    } else {
        "Filesystem entry"
    }
}

pub(crate) fn creation_timestamp(path: &Path) -> String {
    statx(
        CWD,
        path,
        AtFlags::SYMLINK_NOFOLLOW | AtFlags::NO_AUTOMOUNT,
        StatxFlags::BTIME,
    )
    .ok()
    .filter(|value| StatxFlags::from_bits_retain(value.stx_mask).contains(StatxFlags::BTIME))
    .and_then(|value| {
        let seconds = value.stx_btime.tv_sec;
        let nanos = value.stx_btime.tv_nsec;
        (seconds > 0).then_some(UNIX_EPOCH + Duration::new(seconds as u64, nanos))
    })
    .map(timestamp)
    .unwrap_or_default()
}

pub(super) fn marker_from_metadata(path: &Path) -> bool {
    let marker = path.join(".git");
    fs::symlink_metadata(&marker)
        .map(|metadata| metadata.is_dir() || metadata.is_file())
        .unwrap_or(false)
}

pub(super) fn regular_nofollow(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|metadata| metadata.is_file() && !metadata.file_type().is_symlink())
        .unwrap_or(false)
}

pub(super) fn open_regular(path: &Path) -> io::Result<File> {
    let descriptor = open(
        path,
        OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK,
        Mode::empty(),
    )?;
    let file = File::from(descriptor);
    if !file.metadata()?.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Not a regular file",
        ));
    }
    Ok(file)
}

pub(super) fn permissions(mode: u32) -> String {
    let mut result = String::with_capacity(10);
    result.push(match mode & 0o170000 {
        0o040000 => 'd',
        0o120000 => 'l',
        0o140000 => 's',
        0o010000 => 'p',
        0o020000 => 'c',
        0o060000 => 'b',
        _ => '-',
    });
    for (read, write, execute, special, special_char) in [
        (0o400, 0o200, 0o100, 0o4000, 's'),
        (0o040, 0o020, 0o010, 0o2000, 's'),
        (0o004, 0o002, 0o001, 0o1000, 't'),
    ] {
        result.push(if mode & read != 0 { 'r' } else { '-' });
        result.push(if mode & write != 0 { 'w' } else { '-' });
        result.push(match (mode & execute != 0, mode & special != 0) {
            (true, true) => special_char,
            (false, true) => special_char.to_ascii_uppercase(),
            (true, false) => 'x',
            (false, false) => '-',
        });
    }
    result
}

pub(super) fn owner_name(uid: u32) -> String {
    passwd_database()
        .get(&uid)
        .cloned()
        .unwrap_or_else(|| uid.to_string())
}

pub(super) fn group_name(gid: u32) -> String {
    group_database()
        .get(&gid)
        .cloned()
        .unwrap_or_else(|| gid.to_string())
}

pub(super) fn passwd_database() -> &'static HashMap<u32, String> {
    static DATABASE: OnceLock<HashMap<u32, String>> = OnceLock::new();
    DATABASE.get_or_init(|| identity_database(Path::new("/etc/passwd"), IDENTITY_DATABASE_LIMIT))
}

pub(super) fn group_database() -> &'static HashMap<u32, String> {
    static DATABASE: OnceLock<HashMap<u32, String>> = OnceLock::new();
    DATABASE.get_or_init(|| identity_database(Path::new("/etc/group"), IDENTITY_DATABASE_LIMIT))
}

pub(super) fn identity_database(path: &Path, limit: usize) -> HashMap<u32, String> {
    let mut result = HashMap::new();
    let Ok(data) = read_regular_file(path, limit) else {
        return result;
    };
    for line in String::from_utf8_lossy(&data).lines().take(65_536) {
        let mut fields = line.split(':');
        let Some(name) = fields.next() else {
            continue;
        };
        let _ = fields.next();
        let Some(id) = fields.next().and_then(|value| value.parse::<u32>().ok()) else {
            continue;
        };
        result.insert(id, name.chars().take(256).collect());
    }
    result
}

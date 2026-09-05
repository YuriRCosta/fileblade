use super::*;

pub fn validate_save_target(raw_path: &str) -> Value {
    let path = match parse_path(raw_path) {
        Ok(path) => path,
        Err(error) => return path_error(raw_path, &error),
    };
    let parent = path.parent().unwrap_or_else(|| Path::new("/"));
    let path_value = path_text(&path);
    let parent_value = path_text(parent);
    if !parent.is_dir() {
        return json!({
            "ok": false,
            "path": path_value,
            "parent": parent_value,
            "error": "Destination folder does not exist"
        });
    }
    if accessat(
        CWD,
        parent,
        Access::WRITE_OK | Access::EXEC_OK,
        AtFlags::EACCESS,
    )
    .is_err()
    {
        return json!({
            "ok": false,
            "path": path_value,
            "parent": parent_value,
            "error": "Destination folder is not writable"
        });
    }
    let exists = fs::symlink_metadata(&path).is_ok();
    if exists && path.is_dir() {
        return json!({
            "ok": false,
            "path": path_value,
            "parent": parent_value,
            "exists": true,
            "error": "A folder already uses this name"
        });
    }
    json!({
        "ok": true,
        "path": path_value,
        "parent": parent_value,
        "exists": exists,
        "error": ""
    })
}

pub fn read_text(raw_path: &str, limit: usize) -> Value {
    let path = match parse_path(raw_path) {
        Ok(path) => path,
        Err(error) => return path_error(raw_path, &error),
    };
    let maximum = limit.clamp(1, MAX_TEXT_BYTES);
    match read_regular_file(&path, maximum) {
        Ok(data) => match String::from_utf8(data) {
            Ok(text) => json!({
                "ok": true,
                "path": path_text(&path),
                "text": text,
                "bytes": text.len()
            }),
            Err(error) => error_payload(&path, &error),
        },
        Err(error) if error.kind() == io::ErrorKind::FileTooLarge => json!({
            "ok": false,
            "path": path_text(&path),
            "error": format!("File exceeds {maximum} bytes")
        }),
        Err(error) if error.kind() == io::ErrorKind::InvalidInput => json!({
            "ok": false,
            "path": path_text(&path),
            "error": "Not a regular file"
        }),
        Err(error) => error_payload(&path, &error),
    }
}

pub fn content_type(raw_path: &str) -> String {
    content_type_cancellable(raw_path, &AtomicBool::new(false))
}

pub fn content_type_cancellable(raw_path: &str, cancelled: &AtomicBool) -> String {
    let Ok(path) = parse_path(raw_path) else {
        return "application/octet-stream".to_string();
    };
    if cancelled.load(Ordering::Relaxed) {
        return if path.is_dir() {
            "inode/directory".to_string()
        } else {
            entry_mime(
                path.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or_default(),
                false,
            )
        };
    }
    content_type_path_cancellable(&path, cancelled)
}

pub(super) fn content_type_path_cancellable(path: &Path, cancelled: &AtomicBool) -> String {
    if let Some(gio) = which("gio") {
        let output = CommandSpec::new(gio)
            .args([
                "info".to_string(),
                "-a".to_string(),
                "standard::content-type".to_string(),
                path_text(path),
            ])
            .timeout(Duration::from_secs(2))
            .limits(64 * 1024, 16 * 1024)
            .run_cancellable(cancelled);
        if let Ok(completed) = output
            && !completed.stdout_truncated
        {
            for line in String::from_utf8_lossy(&completed.stdout).lines() {
                if let Some(value) = line.trim().strip_prefix("standard::content-type:") {
                    let value = value.trim();
                    if valid_mime(value) {
                        return value.to_string();
                    }
                }
            }
        }
    }
    if path.is_dir() {
        "inode/directory".to_string()
    } else {
        entry_mime(
            path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default(),
            false,
        )
    }
}

pub(super) fn trim_ascii_start(value: &[u8]) -> &[u8] {
    let start = value
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(value.len());
    &value[start..]
}

pub(super) fn mime_database() -> &'static HashMap<Box<str>, Box<str>> {
    static DATABASE: OnceLock<HashMap<Box<str>, Box<str>>> = OnceLock::new();
    DATABASE.get_or_init(|| {
        let mut result: HashMap<Box<str>, Box<str>> = HashMap::from([
            ("txt".into(), "text/plain".into()),
            ("md".into(), "text/markdown".into()),
            ("json".into(), "application/json".into()),
            ("qml".into(), "text/x-qml".into()),
            ("png".into(), "image/png".into()),
            ("jpg".into(), "image/jpeg".into()),
            ("jpeg".into(), "image/jpeg".into()),
            ("gif".into(), "image/gif".into()),
            ("svg".into(), "image/svg+xml".into()),
            ("mp3".into(), "audio/mpeg".into()),
            ("mp4".into(), "video/mp4".into()),
            ("pdf".into(), "application/pdf".into()),
        ]);
        if let Ok(data) = read_regular_file(Path::new("/etc/mime.types"), MIME_DATABASE_LIMIT) {
            for line in String::from_utf8_lossy(&data).lines().take(32_768) {
                let line = line.split('#').next().unwrap_or_default();
                let mut fields = line.split_whitespace();
                let Some(mime) = fields.next().filter(|value| valid_mime(value)) else {
                    continue;
                };
                for extension in fields.take(64) {
                    if extension.len() <= 64
                        && extension.bytes().all(|byte| {
                            byte.is_ascii_alphanumeric()
                                || matches!(byte, b'+' | b'-' | b'.' | b'_')
                        })
                    {
                        let key: Box<str> =
                            if extension.bytes().any(|byte| byte.is_ascii_uppercase()) {
                                extension.to_lowercase().into()
                            } else {
                                extension.into()
                            };
                        result.entry(key).or_insert_with(|| mime.into());
                    }
                }
            }
        }
        result
    })
}

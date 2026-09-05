use super::*;

pub(super) fn command_binary(program: &OsStr) -> AppResult<PathBuf> {
    if let Some(text) = program.to_str() {
        if text.starts_with("file://") {
            return Ok(parse_path(text)?);
        }
        if let Some(binary) = which(text) {
            return Ok(binary);
        }
    }
    Ok(PathBuf::from(program))
}

pub(super) fn quoted_paths(paths: &[String]) -> String {
    paths
        .iter()
        .map(|path| {
            if path.starts_with("file://") {
                parse_path(path)
                    .map(|path| shell_quote_native(path.as_os_str()))
                    .unwrap_or_else(|_| shell_quote(path))
            } else {
                shell_quote(path)
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn shell_quote(value: &str) -> String {
    if !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_@%+=:,./-".contains(&byte))
    {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

pub(super) fn relative_path(base: &Path, path: &Path) -> Option<PathBuf> {
    let base = fs::canonicalize(base).ok()?;
    let path = fs::canonicalize(path).ok()?;
    let base_parts = base.components().collect::<Vec<_>>();
    let path_parts = path.components().collect::<Vec<_>>();
    let common = base_parts
        .iter()
        .zip(&path_parts)
        .take_while(|(left, right)| left == right)
        .count();
    let mut result = PathBuf::new();
    for _ in common..base_parts.len() {
        result.push("..");
    }
    for component in &path_parts[common..] {
        result.push(component.as_os_str());
    }
    Some(result)
}

pub(super) fn command_text(value: &OsStr) -> String {
    value
        .to_str()
        .map(str::to_string)
        .unwrap_or_else(|| display_path(Path::new(value)))
}

pub(super) fn shell_quote_native(value: &OsStr) -> String {
    match value.to_str() {
        Some(text) => shell_quote(text),
        None => format!(
            "$'{}'",
            value
                .as_bytes()
                .iter()
                .map(|byte| format!("\\x{byte:02x}"))
                .collect::<String>()
        ),
    }
}

pub(super) fn native_directory_arg(path: &str) -> AppResult<OsString> {
    let mut argument = OsString::from("--dir=");
    argument.push(parse_path(path)?);
    Ok(argument)
}

pub(super) fn encode_path(value: &Path) -> String {
    value
        .as_os_str()
        .as_bytes()
        .iter()
        .copied()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || b"-._~/".contains(&byte) {
                (byte as char).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_quoting_round_trips_bytes_without_evaluation() {
        let native = OsStr::from_bytes(b"/fixture/\xff'$(false)\n.txt");
        let script = format!("printf '%s' {}", shell_quote_native(native));
        let output = CommandSpec::new("/bin/bash")
            .args(["-c", &script])
            .run()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout, native.as_bytes());
        assert_eq!(
            encode_path(Path::new(OsStr::from_bytes(b"/a/\xff.txt"))),
            "/a/%FF.txt"
        );
    }
}

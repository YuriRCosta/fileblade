use crate::common::path_text;
use crate::secure::ensure_private_directory;
use crate::{AppError, AppResult};
use serde_json::{Value, json};
use std::path::PathBuf;

pub const MAX_MODULE_ID_BYTES: usize = 128;
const STATE_SEGMENT: &str = "modules";
const CONFIG_SEGMENT: &str = "config";

pub fn dir_name(id: &str) -> Option<String> {
    if id.is_empty() || id.len() > MAX_MODULE_ID_BYTES {
        return None;
    }
    let mut segments = id.split('/');
    let first = segments.next()?;
    let second = segments.next();
    if segments.next().is_some() || !valid_segment(first) || !second.is_none_or(valid_segment) {
        return None;
    }
    Some(id.replace('/', "+"))
}

fn valid_segment(segment: &str) -> bool {
    let mut characters = segment.chars();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_alphanumeric())
        && characters.all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
        })
}

pub fn state_root() -> PathBuf {
    crate::paths::state_dir().join(STATE_SEGMENT)
}

pub fn config_root() -> PathBuf {
    crate::paths::config_dir().join(CONFIG_SEGMENT)
}

pub fn state_dir(id: &str) -> Option<PathBuf> {
    dir_name(id).map(|name| state_root().join(name))
}

pub fn config_dir(id: &str) -> Option<PathBuf> {
    dir_name(id).map(|name| config_root().join(name))
}

pub fn ensure(id: &str) -> AppResult<Value> {
    let name = dir_name(id).ok_or_else(|| {
        AppError::invalid(
            "module id must be letters, digits, dot, underscore or dash, with at most one slash and at most 128 bytes",
        )
    })?;
    let state = state_root().join(&name);
    let config = config_root().join(&name);
    ensure_private_directory(&state)?;
    if let Err(error) = ensure_private_directory(&config) {
        let _ = std::fs::remove_dir(&state);
        return Err(error.into());
    }
    Ok(json!({
        "ok": true,
        "module": id,
        "name": name,
        "state_dir": path_text(&state),
        "config_dir": path_text(&config),
    }))
}

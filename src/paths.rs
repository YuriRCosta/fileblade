use std::path::PathBuf;

use crate::common::expanded_path;

const CURRENT: &str = "omarchy/fileblade";
const LEGACY: &str = "omarchy/filetree";

pub fn xdg_home(variable: &str, fallback: &str) -> PathBuf {
    std::env::var_os(variable)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .unwrap_or_else(|| expanded_path(fallback))
}

pub fn state_dir() -> PathBuf {
    migrated(xdg_home("XDG_STATE_HOME", "~/.local/state"))
}

pub fn config_dir() -> PathBuf {
    migrated(xdg_home("XDG_CONFIG_HOME", "~/.config"))
}

fn migrated(home: PathBuf) -> PathBuf {
    let current = home.join(CURRENT);
    if current.symlink_metadata().is_ok() {
        return current;
    }
    let legacy = home.join(LEGACY);
    let legacy_is_plain_dir = legacy
        .symlink_metadata()
        .map(|meta| meta.is_dir())
        .unwrap_or(false);
    if legacy_is_plain_dir {
        let _ = std::fs::rename(&legacy, &current);
    }
    current
}

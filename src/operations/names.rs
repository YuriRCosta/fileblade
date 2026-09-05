use super::*;
use std::os::unix::ffi::{OsStrExt, OsStringExt};

pub fn checked_name(name: &str) -> AppResult<String> {
    if name.trim().is_empty()
        || matches!(name, "." | "..")
        || name.contains('/')
        || name.contains('\0')
    {
        Err(AppError::invalid("Enter a single valid file name"))
    } else {
        Ok(name.to_string())
    }
}

pub(super) fn checked_rename_name(name: &str, escaped: bool) -> AppResult<OsString> {
    if !escaped {
        return Ok(OsString::from(checked_name(name)?));
    }
    let name = crate::common::parse_display_name(name)?;
    let bytes = name.as_bytes();
    if bytes.is_empty()
        || matches!(bytes, b"." | b"..")
        || bytes.contains(&b'/')
        || bytes.contains(&0)
        || bytes.iter().all(u8::is_ascii_whitespace)
    {
        return Err(AppError::invalid("Enter a single valid file name"));
    }
    Ok(name)
}

pub(super) fn unique_copy_target(
    destination: &CheckedDirectory,
    name: &OsStr,
) -> AppResult<secure::ResolvedParent> {
    let candidate = secure::resolved_child(&destination.directory, &destination.path, name)?;
    if !secure::entry_exists_resolved(&candidate)? {
        return Ok(candidate);
    }
    let bytes = name.as_bytes();
    let suffix_start = suffix_start(bytes);
    let (stem, suffix) = suffix_start.map_or((bytes, &[][..]), |index| bytes.split_at(index));
    let mut counter = 1_u64;
    loop {
        let label = if counter == 1 {
            " copy".to_string()
        } else {
            format!(" copy {counter}")
        };
        let candidate_name = OsString::from_vec([stem, label.as_bytes(), suffix].concat());
        let candidate = secure::resolved_child(
            &destination.directory,
            &destination.path,
            candidate_name.as_ref(),
        )?;
        if !secure::entry_exists_resolved(&candidate)? {
            return Ok(candidate);
        }
        counter = counter.saturating_add(1);
    }
}

fn suffix_start(name: &[u8]) -> Option<usize> {
    let start = usize::from(name.starts_with(b"."));
    name[start..]
        .iter()
        .enumerate()
        .find_map(|(index, character)| (*character == b'.').then_some(start + index))
        .filter(|index| *index + 1 < name.len())
}

use super::*;

pub fn checkpoint_transfer(
    kind: &str,
    mapping: &TransferMapping,
    entry_id: &str,
) -> AppResult<String> {
    record_transfer(kind, std::slice::from_ref(mapping), entry_id)
}

pub fn record_transfer(
    kind: &str,
    mappings: &[TransferMapping],
    entry_id: &str,
) -> AppResult<String> {
    if !matches!(kind, "copy" | "move" | "rename") {
        return Err(AppError::invalid(format!(
            "Unknown journal entry kind {kind}"
        )));
    }
    if mappings
        .iter()
        .all(|mapping| mapping.source == mapping.destination)
    {
        return Ok(entry_id.to_string());
    }
    let id = if entry_id.is_empty() {
        new_entry_id()
    } else {
        entry_id.to_string()
    };
    let mut journal = JournalStore::open()?;
    let position = journal.data.undo.iter().position(|entry| entry.id == id);
    let mut changed = false;
    if let Some(position) = position {
        let entry = &mut journal.data.undo[position];
        if entry.kind != kind {
            return Err(AppError::invalid(format!(
                "journal entry {id} is already a {} operation",
                entry.kind
            )));
        }
        for mapping in mappings {
            if mapping.source == mapping.destination
                || entry
                    .items
                    .iter()
                    .any(|item| item.source == mapping.source && item.target == mapping.destination)
            {
                continue;
            }
            entry.items.push(transfer_item(mapping));
            changed = true;
        }
        if changed {
            entry.label = entry_label(kind, &entry.items);
            entry.at = now_text();
        }
    } else {
        let items: Vec<_> = mappings
            .iter()
            .filter(|mapping| mapping.source != mapping.destination)
            .map(transfer_item)
            .collect();
        if items.is_empty() {
            return Ok(id);
        }
        journal.data.undo.push(JournalEntry {
            id: id.clone(),
            kind: kind.to_string(),
            label: entry_label(kind, &items),
            at: now_text(),
            items,
        });
        journal.data.redo.clear();
        changed = true;
    }
    if changed {
        journal.save()?;
    }
    Ok(id)
}

pub fn record_create(path: &str, is_dir: bool, entry_id: &str) -> AppResult<String> {
    let item = JournalItem {
        source: String::new(),
        target: path.to_string(),
        is_dir,
        trash_dir: String::new(),
        trash_name: String::new(),
        before_value: String::new(),
        after_value: String::new(),
        fingerprint: fingerprint(&parse_path(path)?),
        unverified: false,
    };
    record_items("create", vec![item], entry_id)
}

pub fn record_trash(items: &[JournalItem], entry_id: &str) -> AppResult<String> {
    record_items("trash", items.to_vec(), entry_id)
}

pub fn record_colors(
    paths: &[String],
    before: &[String],
    after: &[String],
    entry_id: &str,
) -> AppResult<Value> {
    if paths.is_empty() || paths.len() != before.len() || paths.len() != after.len() {
        return Err(AppError::invalid(
            "Color changes require matching path, before, and after values",
        ));
    }
    if paths.len() > 4096 {
        return Err(AppError::invalid("Too many color changes"));
    }
    let mut seen = BTreeSet::new();
    let mut items = Vec::with_capacity(paths.len());
    for ((path, before_value), after_value) in paths.iter().zip(before).zip(after) {
        if !valid_path(path) || !valid_color(before_value) || !valid_color(after_value) {
            return Err(AppError::invalid("Invalid folder color change"));
        }
        if !seen.insert(path) {
            return Err(AppError::invalid(format!(
                "Duplicate folder color path {path}"
            )));
        }
        if before_value == after_value {
            continue;
        }
        items.push(JournalItem {
            source: String::new(),
            target: path.clone(),
            is_dir: false,
            trash_dir: String::new(),
            trash_name: String::new(),
            before_value: before_value.clone(),
            after_value: after_value.clone(),
            fingerprint: None,
            unverified: false,
        });
    }
    if !items.is_empty() {
        record_items("color", items.clone(), entry_id)?;
    }
    Ok(color_result(&items, false))
}

pub(super) fn transfer_item(mapping: &TransferMapping) -> JournalItem {
    JournalItem {
        source: mapping.source.clone(),
        target: mapping.destination.clone(),
        is_dir: false,
        trash_dir: String::new(),
        trash_name: String::new(),
        before_value: String::new(),
        after_value: String::new(),
        fingerprint: parse_path(&mapping.destination)
            .ok()
            .and_then(|path| fingerprint(&path)),
        unverified: false,
    }
}

pub(super) fn record_items(
    kind: &str,
    items: Vec<JournalItem>,
    entry_id: &str,
) -> AppResult<String> {
    if items.is_empty() {
        return Ok(entry_id.to_string());
    }
    let id = if entry_id.is_empty() {
        new_entry_id()
    } else {
        entry_id.to_string()
    };
    let entry = JournalEntry {
        id: id.clone(),
        kind: kind.to_string(),
        label: entry_label(kind, &items),
        at: now_text(),
        items,
    };
    let mut journal = JournalStore::open()?;
    if let Some(existing) = journal
        .data
        .undo
        .iter_mut()
        .find(|existing| existing.id == id)
    {
        *existing = entry;
    } else {
        journal.data.undo.push(entry);
        journal.data.redo.clear();
    }
    journal.save()?;
    Ok(id)
}

pub(super) fn now_text() -> String {
    Local::now().format("%Y-%m-%dT%H:%M:%S").to_string()
}

pub(super) fn entry_label(kind: &str, items: &[JournalItem]) -> String {
    if kind == "rename"
        && let Some(item) = items.first()
    {
        return format!(
            "Rename {} to {}",
            display_name(&item.source),
            display_name(&item.target)
        );
    }
    if kind == "create"
        && let Some(item) = items.first()
    {
        let prefix = if item.is_dir {
            "New folder "
        } else {
            "New file "
        };
        return format!("{prefix}{}", display_name(&item.target));
    }
    if kind == "color" {
        let verb = if items.iter().all(|item| item.after_value.is_empty()) {
            "Reset color"
        } else {
            "Color"
        };
        return count_label(verb, &entry_paths(items));
    }
    let verb = match kind {
        "copy" => "Copy",
        "move" => "Move",
        "trash" => "Trash",
        other => return count_label(&title(other), &entry_paths(items)),
    };
    count_label(verb, &entry_paths(items))
}

pub(super) fn entry_paths(items: &[JournalItem]) -> Vec<String> {
    items
        .iter()
        .map(|item| {
            if item.target.is_empty() {
                item.source.clone()
            } else {
                item.target.clone()
            }
        })
        .collect()
}

pub(super) fn count_label(verb: &str, paths: &[String]) -> String {
    if paths.len() == 1 {
        format!("{verb} {}", display_name(&paths[0]))
    } else {
        format!("{verb} {} items", paths.len())
    }
}

pub(super) fn display_name(path: &str) -> String {
    parse_path(path)
        .map(|path| crate::common::file_name(&path))
        .unwrap_or_else(|_| path.to_string())
}

pub(super) fn title(value: &str) -> String {
    let mut characters = value.chars();
    match characters.next() {
        Some(first) => first.to_uppercase().collect::<String>() + characters.as_str(),
        None => String::new(),
    }
}

pub(super) fn summary(entry: &JournalEntry) -> Value {
    let paths: Vec<_> = entry_paths(&entry.items).into_iter().take(8).collect();
    json!({
        "id": entry.id,
        "kind": entry.kind,
        "label": entry.label,
        "at": entry.at,
        "ageSeconds": age_seconds(&entry.at),
        "count": entry.items.len(),
        "paths": paths,
    })
}

pub(super) fn age_seconds(stamp: &str) -> i64 {
    NaiveDateTime::parse_from_str(stamp, "%Y-%m-%dT%H:%M:%S")
        .map(|time| (Local::now().naive_local() - time).num_seconds().max(0))
        .unwrap_or(0)
}

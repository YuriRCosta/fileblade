use super::*;

pub(super) fn parse_item(raw: &str) -> Result<ParsedItem, String> {
    if raw.len() > MAX_ITEM_JSON {
        return Err("item description is too large".to_string());
    }
    let value: Value =
        serde_json::from_str(raw).map_err(|_| "item description is not JSON".to_string())?;
    let object = value
        .as_object()
        .ok_or_else(|| "item description needs an id".to_string())?;
    let id = object.get("id").map(value_text).unwrap_or_default();
    if id.is_empty() {
        return Err("item description needs an id".to_string());
    }
    let paths = object.get("paths").cloned().unwrap_or_else(|| json!([]));
    let paths = paths
        .as_array()
        .filter(|paths| paths.iter().all(Value::is_string))
        .ok_or_else(|| "item paths must be a list of strings".to_string())?
        .iter()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect();
    let position = object
        .get("position")
        .and_then(Value::as_u64)
        .filter(|position| *position <= MAX_LIST_ITEMS as u64)
        .map(|position| position as u32);
    let groups = object
        .get("groups")
        .and_then(Value::as_array)
        .map(|groups| {
            groups
                .iter()
                .filter_map(Value::as_str)
                .take(8)
                .map(|group| truncate(group, 120))
                .collect()
        })
        .unwrap_or_default();
    let mut metrics = BTreeMap::new();
    if let Some(values) = object.get("metrics").and_then(Value::as_object) {
        for (key, value) in values.iter().take(32) {
            let bounded = match value {
                Value::String(text) => Value::String(truncate(text, 200)),
                Value::Null | Value::Bool(_) | Value::Number(_) => value.clone(),
                _ => continue,
            };
            metrics.insert(truncate(key, 40), bounded);
        }
    }
    Ok(ParsedItem {
        id,
        name: truncate(&object.get("name").map(value_text).unwrap_or_default(), 120),
        kind: truncate(&object.get("kind").map(value_text).unwrap_or_default(), 40),
        scope: truncate(&object.get("scope").map(value_text).unwrap_or_default(), 40),
        detail: truncate(
            &object.get("detail").map(value_text).unwrap_or_default(),
            200,
        ),
        path: object.get("path").map(value_text).unwrap_or_default(),
        realpath: object.get("realpath").map(value_text).unwrap_or_default(),
        paths,
        payload: object.get("payload").cloned().unwrap_or(Value::Null),
        position,
        groups,
        metrics,
    })
}

pub(super) fn value_text(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Null => "None".to_string(),
        Value::Bool(value) => {
            if *value {
                "True".to_string()
            } else {
                "False".to_string()
            }
        }
        other => other.to_string(),
    }
}

pub(super) fn truncate(value: &str, limit: usize) -> String {
    value.chars().take(limit).collect()
}

pub(super) fn manifest_for(module: &str, parsed: &ParsedItem, items: Vec<StoredItem>) -> Manifest {
    let deleted_at = Local::now();
    let realpath = if parsed.realpath.is_empty() {
        items
            .iter()
            .find(|item| Path::new(&item.stored).components().count() == 2)
            .map(|item| item.source.clone())
            .filter(|source| source != &parsed.path)
            .unwrap_or_default()
    } else {
        parsed.realpath.clone()
    };
    Manifest {
        schema_version: SCHEMA_VERSION,
        module: module.to_string(),
        id: truncate(&parsed.id, 120),
        name: parsed.name.clone(),
        kind: parsed.kind.clone(),
        scope: parsed.scope.clone(),
        detail: parsed.detail.clone(),
        path: parsed.path.clone(),
        realpath,
        deleted_at: deleted_at.format("%Y-%m-%d %H:%M").to_string(),
        deleted_at_epoch: Some(deleted_at.timestamp()),
        payload: parsed.payload.clone(),
        restore_helper: None,
        position: parsed.position,
        groups: parsed.groups.clone(),
        metrics: parsed.metrics.clone(),
        items,
        restore_completed: Vec::new(),
    }
}

pub(super) fn manifest_link_target(manifest: &Manifest) -> String {
    if !manifest.realpath.is_empty() && manifest.realpath != manifest.path {
        return manifest.realpath.clone();
    }
    manifest
        .items
        .iter()
        .find(|item| item.kind == "symlink" && !item.target.is_empty())
        .and_then(|item| {
            let target = item.link_target();
            if target.is_absolute() {
                Some(path_text(&target))
            } else {
                Some(path_text(
                    &parse_path(&item.source).ok()?.parent()?.join(target),
                ))
            }
        })
        .unwrap_or_default()
}

pub(super) fn manifest_metrics(manifest: &Manifest, stored: Option<&Path>) -> Value {
    if manifest.metrics.is_empty() {
        file_metrics(stored)
    } else {
        json!(manifest.metrics)
    }
}

pub(super) fn manifest_deletion_epoch(manifest: &Manifest) -> Option<i64> {
    manifest.deleted_at_epoch.or_else(|| {
        NaiveDateTime::parse_from_str(&manifest.deleted_at, "%Y-%m-%d %H:%M")
            .ok()?
            .and_local_timezone(Local)
            .single()
            .map(|value| value.timestamp())
    })
}

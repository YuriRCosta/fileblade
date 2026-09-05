use super::*;
use crate::module_helpers::{self, Route};

pub fn remove_with_helper(
    module: &str,
    raw_item: &str,
    raw_route: &str,
    arguments: &str,
    cancelled: &AtomicBool,
) -> Value {
    match prepare_and_remove(module, raw_item, raw_route, arguments, cancelled) {
        Ok(response) => response,
        Err(error) => refusal(error, ""),
    }
}

fn parse_route(raw: &str) -> Result<Route, String> {
    if raw.len() > MAX_ITEM_JSON {
        return Err("recovery helper route is too large".to_string());
    }
    serde_json::from_str(raw).map_err(|_| "invalid recovery helper route".to_string())
}

fn prepare_and_remove(
    module: &str,
    raw_item: &str,
    raw_route: &str,
    arguments: &str,
    cancelled: &AtomicBool,
) -> Result<Value, String> {
    if !valid_module(module) {
        return Err("invalid bin module name".to_string());
    }
    let parsed = parse_item(raw_item)?;
    if !parsed.paths.is_empty() || !parsed.payload.is_null() {
        return Err(
            "helper recovery needs metadata without paths or a supplied payload".to_string(),
        );
    }
    let route = parse_route(raw_route)?;
    for (method, write) in [
        ("prepare-remove", false),
        ("remove-prepared", true),
        ("restore", true),
    ] {
        route
            .validate(method, write)
            .map_err(|error| error.to_string())?;
    }
    let _lease = mutation_lease()?;
    let prepared = module_helpers::run(
        &route.request("prepare-remove", arguments, None, false),
        cancelled,
    )
    .map_err(|error| error.to_string())?;
    if prepared["ok"] != true {
        return Ok(helper_failure(&prepared, "Could not prepare recovery", ""));
    }
    if prepared["schemaVersion"] != SCHEMA_VERSION || !prepared["payload"].is_object() {
        return Err("helper returned no complete recovery record".to_string());
    }
    let payload = prepared["payload"].clone();
    let input = serde_json::to_string(&payload).map_err(|error| error.to_string())?;
    if input.len() > module_helpers::INPUT_LIMIT {
        return Err(
            "recovery payload exceeds the helper input limit; source was not changed".to_string(),
        );
    }
    let mut complete: Value = serde_json::from_str(raw_item).map_err(|error| error.to_string())?;
    complete["payload"] = payload;
    complete["restoreHelper"] = serde_json::to_value(&route).map_err(|error| error.to_string())?;
    // Measure the entire UTF-8 wire record, including metadata and JSON escaping.
    let encoded = serde_json::to_string(&complete).map_err(|error| error.to_string())?;
    let parsed = parse_item(&encoded).map_err(|_| {
        "complete recovery record exceeds 64 KiB; source was not changed".to_string()
    })?;
    let mut commit_args: Vec<String> =
        serde_json::from_str(arguments).map_err(|_| "invalid recovery arguments".to_string())?;
    commit_args.push("--payload-stdin".to_string());
    let commit_args = serde_json::to_string(&commit_args).map_err(|error| error.to_string())?;
    module_helpers::validate_arguments(&commit_args).map_err(|error| error.to_string())?;
    check_cancelled(cancelled).map_err(|error| error.to_string())?;
    let root = bin_root(module).map_err(|error| error.to_string())?;
    let entry_dir = prepare_entry(&root)?;
    let entry_id = format!(
        "{BIN_PREFIX}{}",
        entry_dir.file_name().unwrap().to_string_lossy()
    );
    let mut manifest = manifest_for(module, &parsed, Vec::new());
    manifest.restore_helper = Some(route.clone());
    if let Err(error) = store_items(&entry_dir, &manifest, cancelled) {
        let _ = discard_entry(&entry_dir, Some(&manifest));
        return Err(format!(
            "could not save recovery; source was not changed: {error}"
        ));
    }
    // From here on even an absent helper response can mean the source changed.
    // Never delete this durable record as compensation for a failed request.
    let removed = module_helpers::run(
        &route.request("remove-prepared", &commit_args, Some(&input), true),
        cancelled,
    );
    match removed {
        Ok(response) if response["ok"] == true && response["schemaVersion"] == SCHEMA_VERSION => {
            Ok(document(
                vec![result("", true, true, "disabled; recovery saved")],
                &entry_id,
                Value::Null,
            ))
        }
        Ok(response) => Ok(helper_failure(
            &response,
            "Removal was not confirmed; recovery is saved. Use Restore to reconcile the source",
            &entry_id,
        )),
        Err(_) => Ok(helper_failure(
            &Value::Null,
            "Removal was not confirmed; recovery is saved. Use Restore to reconcile the source",
            &entry_id,
        )),
    }
}

pub fn restore_with_helper(
    module: &str,
    entry_id: &str,
    raw_route: Option<&str>,
    cancelled: &AtomicBool,
) -> Value {
    let helper = match raw_route.map(parse_route).transpose() {
        Ok(helper) => helper,
        Err(error) => return refusal(error, ""),
    };
    let _lease = match mutation_lease() {
        Ok(lease) => lease,
        Err(error) => return refusal(error, ""),
    };
    restore_entry(module, entry_id, helper, cancelled)
}

pub(super) fn restore_payload(
    entry_dir: &Path,
    entry_id: &str,
    manifest: &mut Manifest,
    helper: Option<Route>,
    cancelled: &AtomicBool,
) -> Value {
    let route = match (helper, manifest.restore_helper.as_ref()) {
        (Some(route), Some(saved))
            if route.provider != saved.provider || route.helper != saved.helper =>
        {
            return refusal(
                "the recovery helper does not match this record's provider",
                "",
            );
        }
        (Some(route), _) => route,
        (None, Some(route)) => route.clone(),
        (None, None) => return refusal("this recovery record needs its owning companion", ""),
    };
    let input = match serde_json::to_string(&manifest.payload) {
        Ok(input) if input.len() <= module_helpers::INPUT_LIMIT => input,
        _ => {
            return refusal(
                "recovery payload exceeds the helper input limit; the record was kept",
                "",
            );
        }
    };
    if let Err(error) = route.validate("restore", true) {
        return refusal(
            format!("Recovery helper unavailable; the record was kept: {error}"),
            "",
        );
    }
    manifest.restore_helper = Some(route.clone());
    if let Err(error) = save_manifest(entry_dir, manifest) {
        return refusal(format!("Could not save the recovery helper: {error}"), "");
    }
    let arguments = json!(["--record-id", entry_id, "--payload-stdin", "--json"]).to_string();
    let response = module_helpers::run(
        &route.request("restore", &arguments, Some(&input), true),
        cancelled,
    );
    match response {
        Ok(response) if response["ok"] == true && response["schemaVersion"] == SCHEMA_VERSION => {
            if let Err(error) = discard_entry(entry_dir, Some(manifest)) {
                return helper_failure(
                    &Value::Null,
                    &format!("Source restored but recovery cleanup failed; retry Restore: {error}"),
                    entry_id,
                );
            }
            document(
                vec![result("", true, true, "restored")],
                entry_id,
                Value::Null,
            )
        }
        Ok(response) => helper_failure(
            &response,
            "Restore was not confirmed; the recovery record was kept",
            entry_id,
        ),
        Err(_) => helper_failure(
            &Value::Null,
            "Restore was not confirmed; the recovery record was kept",
            entry_id,
        ),
    }
}

fn helper_failure(response: &Value, fallback: &str, entry: &str) -> Value {
    let detail = response
        .get("message")
        .or_else(|| response.get("error"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let message = if detail.is_empty() {
        fallback.to_string()
    } else {
        format!("{fallback}: {}", truncate(detail, 200))
    };
    document(vec![result("", false, false, message)], entry, Value::Null)
}

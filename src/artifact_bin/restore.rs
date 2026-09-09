use super::*;

pub fn restore(module: &str, entry_id: &str, cancelled: &AtomicBool) -> Value {
    restore_with_helper(module, entry_id, None, cancelled)
}

pub(super) fn restore_entry(
    module: &str,
    entry_id: &str,
    helper: Option<crate::module_helpers::Route>,
    cancelled: &AtomicBool,
) -> Value {
    let entry_dir = match entry_dir_for(module, entry_id) {
        Ok(entry_dir) => entry_dir,
        Err(error) => return refusal(error, ""),
    };
    let (mut manifest, _) = match load_manifest(&entry_dir, Some(module)) {
        Ok(Some(loaded)) => loaded,
        _ => {
            return refusal(
                format!("{} is unreadable", entry_dir.join(MANIFEST_NAME).display()),
                "",
            );
        }
    };
    let roots = root_items(&manifest);
    if roots.is_empty() && (manifest.restore_helper.is_some() || helper.is_some()) {
        return super::transaction::restore_payload(
            &entry_dir,
            entry_id,
            &mut manifest,
            helper,
            cancelled,
        );
    }
    let mut results = Vec::new();
    for root in &roots {
        if let Err(error) = check_cancelled(cancelled) {
            results.push(result(&root.source, false, false, error.to_string()));
            break;
        }
        let subtree = subtree_items(&manifest, &root.stored);
        let destination = match parse_path(&root.source) {
            Ok(path) => path,
            Err(error) => {
                results.push(result(&root.source, false, false, error.to_string()));
                break;
            }
        };
        let destination_target = match restore_target(&destination) {
            Ok(target) => target,
            Err(error) => {
                results.push(result(
                    &root.source,
                    false,
                    false,
                    format!("cannot prepare restore destination: {error}"),
                ));
                break;
            }
        };
        let exists = secure::entry_exists_resolved(&destination_target).unwrap_or(false);
        let exact =
            exists && subtree_matches(&entry_dir, root, &subtree, cancelled).unwrap_or(false);
        let changed = if exists {
            if !exact {
                results.push(result(
                    &root.source,
                    false,
                    false,
                    "something already exists there; move it first",
                ));
                break;
            }
            false
        } else {
            match restore_root(&entry_dir, root, &subtree, &destination_target, cancelled) {
                Ok(()) => true,
                Err(error) => {
                    results.push(result(
                        &root.source,
                        false,
                        false,
                        format!("cannot restore: {error}"),
                    ));
                    break;
                }
            }
        };
        if !manifest.restore_completed.contains(&root.stored) {
            manifest.restore_completed.push(root.stored.clone());
            if let Err(error) = save_manifest(&entry_dir, &manifest) {
                results.push(result(
                    &root.source,
                    false,
                    changed,
                    format!("cannot checkpoint restore: {error}"),
                ));
                break;
            }
        }
        results.extend(subtree.iter().map(|item| {
            result(
                &item.source,
                true,
                changed,
                if changed {
                    "restored"
                } else {
                    "already restored"
                },
            )
        }));
    }
    let requires_commit = roots.is_empty();
    if requires_commit && results.is_empty() {
        results.push(result("", true, true, "restored"));
    }
    if !requires_commit
        && results.iter().all(|item| item.ok)
        && let Err(error) = discard_entry(&entry_dir, Some(&manifest))
    {
        results.push(result(&path_text(&entry_dir), false, false, error));
    }
    let mut response = document(results, entry_id, manifest.payload.clone());
    response["pending_commit"] =
        Value::Bool(requires_commit && response["ok"].as_bool().unwrap_or(false));
    response
}

pub fn purge(module: &str, entry_id: &str) -> Value {
    let _lease = match mutation_lease() {
        Ok(lease) => lease,
        Err(error) => return refusal(error, ""),
    };
    let entry_dir = match entry_dir_for(module, entry_id) {
        Ok(entry_dir) => entry_dir,
        Err(error) => return refusal(error, ""),
    };
    let manifest = load_manifest(&entry_dir, Some(module)).ok().flatten();
    let payload = manifest
        .as_ref()
        .map(|(manifest, _)| manifest.payload.clone())
        .unwrap_or(Value::Null);
    if let Err(error) = discard_entry(&entry_dir, manifest.as_ref().map(|(manifest, _)| manifest)) {
        return refusal(error, &path_text(&entry_dir));
    }
    document(
        vec![result(
            &path_text(&entry_dir),
            true,
            true,
            "deleted forever",
        )],
        entry_id,
        payload,
    )
}

pub(super) fn verify_inventory_item(item: &StoredItem) -> io::Result<()> {
    let path = parse_path(&item.source)?;
    let stat = secure::entry_stat(&path)?;
    let expected_kind = kind_value(&item.kind);
    if stat.kind != expected_kind
        || item.dev.is_some_and(|value| value != stat.dev)
        || item.ino.is_some_and(|value| value != stat.ino)
        || (item.kind == "file" && stat.size != item.size)
        || item
            .mtime_ns
            .is_some_and(|value| value != nanoseconds(stat.mtime, stat.mtime_nsec))
    {
        return Err(io::Error::other(format!(
            "{} changed after it was inspected",
            path.display()
        )));
    }
    if item.kind == "symlink" && secure::read_link(&path)? != item.link_target() {
        return Err(io::Error::other(format!(
            "{} changed after it was inspected",
            path.display()
        )));
    }
    Ok(())
}

pub(super) fn remove_listed(items: &[StoredItem], cancelled: &AtomicBool) -> Vec<ResultRow> {
    for item in items {
        if let Err(error) = check_cancelled(cancelled).and_then(|()| verify_inventory_item(item)) {
            return vec![result(
                &item.source,
                false,
                false,
                format!("cannot remove: {error}"),
            )];
        }
    }
    let mut results = Vec::new();
    for item in items.iter().rev() {
        if let Err(error) = check_cancelled(cancelled) {
            results.push(result(&item.source, false, false, error.to_string()));
            break;
        }
        let expected = match verify_removal_identity(item) {
            Ok(stat) => stat.identity(),
            Err(error) => {
                results.push(result(
                    &item.source,
                    false,
                    false,
                    format!("cannot remove: {error}"),
                ));
                continue;
            }
        };
        let removed = parse_path(&item.source).and_then(|path| {
            if item.kind == "dir" {
                secure::remove_empty_directory_matching(&path, expected)
            } else {
                secure::remove_nondirectory_matching(&path, expected)
            }
        });
        match removed {
            Ok(()) => results.push(result(&item.source, true, true, "removed")),
            Err(error) => results.push(result(
                &item.source,
                false,
                false,
                format!("cannot remove: {error}"),
            )),
        }
    }
    results
}

pub(super) fn verify_removal_identity(item: &StoredItem) -> io::Result<secure::EntryStat> {
    let path = parse_path(&item.source)?;
    let stat = secure::entry_stat(&path)?;
    if stat.kind != kind_value(&item.kind)
        || item.dev.is_some_and(|value| value != stat.dev)
        || item.ino.is_some_and(|value| value != stat.ino)
        || (item.kind == "file" && stat.size != item.size)
        || (item.kind != "dir"
            && item
                .mtime_ns
                .is_some_and(|value| value != nanoseconds(stat.mtime, stat.mtime_nsec)))
    {
        return Err(io::Error::other(format!(
            "{} changed after it was inspected",
            path.display()
        )));
    }
    if item.kind == "symlink" && secure::read_link(&path)? != item.link_target() {
        return Err(io::Error::other(format!(
            "{} changed after it was inspected",
            path.display()
        )));
    }
    Ok(stat)
}

pub(super) fn root_items(manifest: &Manifest) -> Vec<StoredItem> {
    manifest
        .items
        .iter()
        .filter(|item| Path::new(&item.stored).components().count() == 2)
        .cloned()
        .collect()
}

pub(super) fn subtree_items(manifest: &Manifest, root_stored: &str) -> Vec<StoredItem> {
    let root = Path::new(root_stored);
    let mut items: Vec<_> = manifest
        .items
        .iter()
        .filter(|item| Path::new(&item.stored).starts_with(root))
        .cloned()
        .collect();
    items.sort_by_key(|item| Path::new(&item.stored).components().count());
    items
}

pub(super) fn restore_root(
    entry_dir: &Path,
    root: &StoredItem,
    subtree: &[StoredItem],
    destination: &secure::ResolvedParent,
    cancelled: &AtomicBool,
) -> io::Result<()> {
    let stage = entry_dir.join(format!(".fileblade-restore-{}", Uuid::new_v4().simple()));
    secure::create_directory_noreplace(&stage, PRIVATE_DIRECTORY_MODE)?;
    let partial = stage.join("item");
    let outcome = (|| {
        for item in subtree {
            check_cancelled(cancelled)?;
            let target = mapped_destination(item, root, &partial)?;
            let stored = entry_dir.join(&item.stored);
            match item.kind.as_str() {
                "dir" => {
                    secure::create_directory_noreplace(&target, PRIVATE_DIRECTORY_MODE)?;
                    copy_directory_xattrs(&stored, &target)?;
                }
                "symlink" => {
                    let link = secure::read_link(&stored)?;
                    if link != item.link_target() {
                        return Err(io::Error::other(format!(
                            "{} no longer matches its manifest",
                            stored.display()
                        )));
                    }
                    secure::create_symlink_noreplace(&target, &link)?;
                }
                "file" => copy_regular_private(&stored, &target, item.size, cancelled)?,
                _ => return Err(io::Error::other("invalid stored item type")),
            }
        }
        apply_restored_metadata(root, subtree, &partial)?;
        let expected = secure::entry_stat(&partial)?.identity();
        let mut ignored: fn(&Path) = |_| {};
        let source = secure::resolved_parent(&partial)?;
        secure::relocate_noreplace_matching_resolved(
            source,
            destination,
            expected,
            cancelled,
            &mut ignored,
        )?;
        secure::remove_empty_directory(&stage)
    })();
    if outcome.is_err() {
        let _ = secure::remove_path(&stage);
    }
    outcome
}

pub(super) fn restore_target(destination: &Path) -> io::Result<secure::ResolvedParent> {
    let parent = destination
        .parent()
        .ok_or_else(|| io::Error::other(format!("{} has no parent", destination.display())))?;
    let name = destination
        .file_name()
        .ok_or_else(|| io::Error::other(format!("{} has no file name", destination.display())))?;
    let directory = secure::ensure_directories(parent, 0o777)?;
    secure::resolved_child(&directory, parent, name)
}

pub(super) fn mapped_destination(
    item: &StoredItem,
    root: &StoredItem,
    base: &Path,
) -> io::Result<PathBuf> {
    let source = parse_path(&item.source)?;
    let root = parse_path(&root.source)?;
    let relative = source
        .strip_prefix(root)
        .map_err(|_| io::Error::other("stored item is outside its root"))?;
    Ok(if relative.as_os_str().is_empty() {
        base.to_path_buf()
    } else {
        base.join(relative)
    })
}

pub(super) fn subtree_matches(
    entry_dir: &Path,
    root: &StoredItem,
    subtree: &[StoredItem],
    cancelled: &AtomicBool,
) -> io::Result<bool> {
    for item in subtree {
        check_cancelled(cancelled)?;
        let destination = mapped_destination(item, root, &parse_path(&root.source)?)?;
        let stat = match secure::entry_stat(&destination) {
            Ok(stat) => stat,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(error),
        };
        if stat.kind != kind_value(&item.kind) {
            return Ok(false);
        }
        match item.kind.as_str() {
            "file" => {
                if stat.size != item.size
                    || !files_equal(
                        &entry_dir.join(&item.stored),
                        &destination,
                        item.size,
                        cancelled,
                    )?
                {
                    return Ok(false);
                }
            }
            "symlink" => {
                if secure::read_link(&destination)? != item.link_target()
                    || secure::read_link(&entry_dir.join(&item.stored))? != item.link_target()
                {
                    return Ok(false);
                }
            }
            "dir" => {
                let expected: BTreeSet<_> = subtree
                    .iter()
                    .filter(|candidate| {
                        Path::new(&candidate.stored).parent() == Some(Path::new(&item.stored))
                    })
                    .filter_map(|candidate| {
                        parse_path(&candidate.source)
                            .ok()?
                            .file_name()
                            .map(ToOwned::to_owned)
                    })
                    .collect();
                let actual: BTreeSet<_> =
                    secure::directory_names(&destination)?.into_iter().collect();
                if actual != expected {
                    return Ok(false);
                }
            }
            _ => return Ok(false),
        }
    }
    Ok(true)
}

pub(super) fn files_equal(
    left: &Path,
    right: &Path,
    expected_size: u64,
    cancelled: &AtomicBool,
) -> io::Result<bool> {
    let mut left = secure::open_file_read(left)?;
    let mut right = secure::open_file_read(right)?;
    if left.metadata()?.len() != expected_size || right.metadata()?.len() != expected_size {
        return Ok(false);
    }
    let mut left_buffer = [0_u8; COPY_CHUNK];
    let mut right_buffer = [0_u8; COPY_CHUNK];
    loop {
        check_cancelled(cancelled)?;
        let left_count = left.read(&mut left_buffer)?;
        let right_count = right.read(&mut right_buffer)?;
        if left_count != right_count || left_buffer[..left_count] != right_buffer[..right_count] {
            return Ok(false);
        }
        if left_count == 0 {
            return Ok(true);
        }
    }
}

pub(super) fn discard_entry(entry_dir: &Path, manifest: Option<&Manifest>) -> Result<(), String> {
    if let Some(manifest) = manifest {
        super::transaction::discard_helper_record(entry_dir, manifest)?;
        for item in manifest.items.iter().rev() {
            let path = entry_dir.join(&item.stored);
            let outcome = if item.kind == "dir" {
                secure::remove_empty_directory(&path)
            } else {
                secure::remove_nondirectory(&path)
            };
            if let Err(error) = outcome {
                return Err(format!("cannot remove {}: {error}", path.display()));
            }
        }
    }
    for (directory, path) in [
        (false, entry_dir.join(MANIFEST_NAME)),
        (true, entry_dir.join(ITEMS_DIRECTORY)),
        (true, entry_dir.to_path_buf()),
    ] {
        let outcome = if directory {
            secure::remove_empty_directory(&path)
        } else {
            secure::remove_nondirectory(&path)
        };
        if let Err(error) = outcome {
            return Err(format!("cannot remove {}: {error}", path.display()));
        }
    }
    Ok(())
}

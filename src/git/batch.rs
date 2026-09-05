use super::*;

pub fn git_repositories_for_markers(
    markers: &[PathBuf],
) -> HashMap<PathBuf, Option<GitRepository>> {
    git_repositories_for_markers_cancellable(markers, &AtomicBool::new(false))
}

pub fn git_repositories_for_markers_cancellable(
    markers: &[PathBuf],
    cancelled: &AtomicBool,
) -> HashMap<PathBuf, Option<GitRepository>> {
    repositories_until(markers, cancelled, None, None)
}

pub fn git_repositories_for_markers_bounded(
    markers: &[PathBuf],
    cancelled: &AtomicBool,
    timeout: Duration,
) -> HashMap<PathBuf, Option<GitRepository>> {
    repositories_until(
        markers,
        cancelled,
        Instant::now().checked_add(timeout),
        None,
    )
}

pub(crate) fn cached_git_repositories_for_markers_bounded(
    markers: &[PathBuf],
    cancelled: &AtomicBool,
    timeout: Duration,
    refresh: bool,
) -> HashMap<PathBuf, Option<GitRepository>> {
    repositories_until(
        markers,
        cancelled,
        Instant::now().checked_add(timeout),
        Some(refresh),
    )
}

fn repositories_until(
    markers: &[PathBuf],
    cancelled: &AtomicBool,
    deadline: Option<Instant>,
    cache_refresh: Option<bool>,
) -> HashMap<PathBuf, Option<GitRepository>> {
    let mut unique = Vec::new();
    let mut seen = HashSet::new();
    for marker in markers.iter().take(MAX_GIT_MARKERS) {
        if seen.insert(marker.clone()) {
            unique.push(marker.clone());
        }
    }
    let mut result = HashMap::new();
    let mut aggregate = 0_usize;
    let mut aggregate_exhausted = false;
    for chunk in unique.chunks(4) {
        if cancelled.load(Ordering::Relaxed) {
            break;
        }
        let status_timeout = deadline
            .map(|value| value.saturating_duration_since(Instant::now()))
            .unwrap_or(Duration::from_secs(4))
            .min(Duration::from_secs(4));
        if status_timeout.is_zero() {
            break;
        }
        let repositories = std::thread::scope(|scope| {
            chunk
                .iter()
                .map(|marker| {
                    scope.spawn(move || {
                        if let Some(refresh) = cache_refresh {
                            cached_git_worktree_status(marker, refresh, status_timeout, cancelled)
                        } else {
                            git_worktree_status_with_timeout(
                                &path_text(marker),
                                true,
                                status_timeout,
                                cancelled,
                            )
                        }
                    })
                })
                .collect::<Vec<_>>()
                .into_iter()
                .map(|worker| worker.join().unwrap_or(None))
                .collect::<Vec<_>>()
        });
        for (marker, mut repository) in chunk.iter().cloned().zip(repositories) {
            if let Some(value) = repository.as_mut() {
                let weight = repository_weight(value);
                if aggregate_exhausted || aggregate.saturating_add(weight) > MAX_GIT_AGGREGATE_BYTES
                {
                    aggregate_exhausted = true;
                    value.ok = false;
                    value.dirty = false;
                    value.entries.clear();
                    value.error = "Git status aggregate exceeds 16 MiB".to_string();
                } else {
                    aggregate += weight;
                }
            }
            result.insert(marker, repository);
        }
        if aggregate_exhausted || aggregate >= MAX_GIT_AGGREGATE_BYTES {
            break;
        }
    }
    result
}

pub fn git_metadata_batch(paths: &[String]) -> Value {
    git_metadata_batch_cancellable(paths, &AtomicBool::new(false))
}

pub fn git_metadata_batch_cancellable(paths: &[String], cancelled: &AtomicBool) -> Value {
    let mut unique = Vec::new();
    let mut seen = HashSet::new();
    for raw_path in paths.iter().take(MAX_GIT_BATCH_PATHS) {
        let path = match parse_path(raw_path) {
            Ok(path) => path,
            Err(error) => return path_error(raw_path, &error),
        };
        if seen.insert(path.clone()) {
            unique.push(path);
        }
    }
    let mut marker_cache = HashMap::new();
    let markers = unique
        .iter()
        .map(|path| nearest_git_marker(&path_text(path), &mut marker_cache))
        .collect::<Vec<_>>();
    let repository_markers = markers.iter().flatten().cloned().collect::<Vec<_>>();
    let repositories_by_marker = cached_git_repositories_for_markers_bounded(
        &repository_markers,
        cancelled,
        Duration::from_secs(4),
        true,
    );
    if cancelled.load(Ordering::Relaxed) {
        return json!({
            "ok": false,
            "repositories": 0,
            "results": [],
            "error": "operation cancelled"
        });
    }
    let mut ignored_by_root: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
    for (path, marker) in unique.iter().zip(markers.iter()) {
        if let Some(repository) = marker
            .as_ref()
            .and_then(|value| repositories_by_marker.get(value))
            .and_then(Option::as_ref)
        {
            ignored_by_root
                .entry(repository.root.clone())
                .or_default()
                .push(path.clone());
        }
    }
    let mut ignored = HashSet::new();
    for (root, paths) in ignored_by_root {
        ignored.extend(excludes::ignored_paths(&root, &paths, cancelled));
    }
    let mut repositories = HashSet::new();
    let mut indexes = HashMap::new();
    let mut results = Vec::with_capacity(unique.len());
    for (path, marker) in unique.into_iter().zip(markers) {
        let exists = std::fs::symlink_metadata(&path).is_ok();
        let is_dir = path.is_dir();
        let mut response = json!({
            "ok": true,
            "path": path_text(&path),
            "exists": exists,
            "is_dir": is_dir
        });
        let repository = marker
            .as_ref()
            .and_then(|value| repositories_by_marker.get(value))
            .and_then(Option::as_ref);
        if let Some(repository) = repository {
            repositories.insert(repository.root.clone());
            let index = match indexes.entry(repository.root.clone()) {
                std::collections::hash_map::Entry::Occupied(entry) => entry.into_mut(),
                std::collections::hash_map::Entry::Vacant(entry) => {
                    let Some(index) = repository_status_index_cancellable(repository, cancelled)
                    else {
                        return json!({
                            "ok": false,
                            "repositories": 0,
                            "results": [],
                            "error": "operation cancelled"
                        });
                    };
                    entry.insert(index)
                }
            };
            let status = indexed_git_status_for_path(index, &path, is_dir);
            let counts = indexed_git_counts_for_path(index, &path, is_dir);
            response["git"] = git_metadata_document(repository, status, is_dir, counts);
            response["git"]["ignored"] = json!(ignored.contains(&path));
        }
        results.push(response);
    }
    json!({
        "ok": true,
        "repositories": repositories.len(),
        "results": results
    })
}

use super::*;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

const CACHE_CAPACITY: usize = 64;
#[derive(Debug)]
struct CacheEntry {
    repository: Option<GitRepository>,
    last_used: Instant,
}

type Cache = Mutex<HashMap<PathBuf, Arc<Mutex<CacheEntry>>>>;

fn registry() -> &'static Cache {
    static CACHE: OnceLock<Cache> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn slot(root: &Path) -> Arc<Mutex<CacheEntry>> {
    let mut entries = lock(registry());
    if let Some(entry) = entries.get(root) {
        return Arc::clone(entry);
    }
    if entries.len() >= CACHE_CAPACITY {
        let oldest = entries
            .iter()
            .filter(|(_, entry)| Arc::strong_count(entry) == 1)
            .min_by_key(|(_, entry)| lock(entry).last_used)
            .map(|(root, _)| root.clone());
        if let Some(root) = oldest {
            entries.remove(&root);
        }
    }
    let entry = Arc::new(Mutex::new(CacheEntry {
        repository: None,
        last_used: Instant::now(),
    }));
    entries.insert(root.to_path_buf(), Arc::clone(&entry));
    entry
}

pub(super) fn repository(
    marker: &Path,
    refresh: bool,
    timeout: Duration,
    cancelled: &AtomicBool,
) -> Option<GitRepository> {
    let shell = repository_from_marker(&path_text(marker))?;
    let entry = slot(&shell.root);
    let mut entry = lock(&entry);
    entry.last_used = Instant::now();
    if let Some(repository) = entry.repository.as_ref()
        && !refresh
    {
        return Some(repository.clone());
    }

    let previous = entry.repository.clone();
    let loaded = load_repository_status(shell, timeout, cancelled)?;
    if loaded.ok {
        entry.repository = Some(loaded.clone());
        Some(loaded)
    } else {
        previous
            .map(|mut repository| {
                repository.error = loaded.error.clone();
                repository
            })
            .or(Some(loaded))
    }
}

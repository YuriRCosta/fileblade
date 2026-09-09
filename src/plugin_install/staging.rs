use crate::{AppResult, secure};
use std::path::{Path, PathBuf};

pub(super) struct Staging {
    pub path: PathBuf,
    identity: secure::EntryIdentity,
    intent: secure::RecoveryIntent,
}

impl Staging {
    pub fn create(plugins: &Path) -> AppResult<Self> {
        let path = plugins.parent().unwrap_or(plugins).join(format!(
            ".fileblade-partial-{}",
            uuid::Uuid::new_v4().simple()
        ));
        secure::create_directory_noreplace(&path, 0o700)?;
        let identity = secure::entry_stat(&path)?.identity();
        let intent = match secure::write_intent(&serde_json::json!({
            "kind": "partial", "partial": crate::common::path_text(&path),
            "dev": identity.dev, "ino": identity.ino
        })) {
            Ok(intent) => intent,
            Err(error) => {
                let _ = secure::remove_empty_directory_matching(&path, identity);
                return Err(error.into());
            }
        };
        Ok(Self {
            path,
            identity,
            intent,
        })
    }

    pub fn clear(&self) -> AppResult<()> {
        if secure::entry_exists(&self.path)? {
            secure::remove_path_matching(&self.path, self.identity)?;
        }
        self.intent.clear()?;
        Ok(())
    }
}

impl Drop for Staging {
    fn drop(&mut self) {
        let _ = self.clear();
    }
}

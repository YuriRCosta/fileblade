use super::*;

pub struct RecoveryIntent {
    path: PathBuf,
    _lease: LockedFile,
}

impl RecoveryIntent {
    pub fn clear(&self) -> io::Result<()> {
        remove_nondirectory(&self.path)
    }
}

pub fn write_intent(record: &serde_json::Value) -> io::Result<RecoveryIntent> {
    let directory = crate::recovery::inflight_dir();
    ensure_private_directory(&directory)?;
    let path = directory.join(format!("{}.json", Uuid::new_v4().simple()));
    let temporary = path.with_extension("tmp");
    let mut file = create_file_noreplace(&temporary, PRIVATE_FILE_MODE)?;
    let result = (|| {
        flock(&file, FlockOperation::LockExclusive).map_err(io::Error::from)?;
        file.write_all(record.to_string().as_bytes())?;
        file.sync_all()?;
        rename_noreplace(&temporary, &path)
    })();
    if let Err(error) = result {
        let _ = remove_nondirectory(&temporary);
        let _ = remove_nondirectory(&path);
        return Err(error);
    }
    Ok(RecoveryIntent {
        path,
        _lease: LockedFile { file },
    })
}

pub fn try_lock_private(path: &Path) -> io::Result<Option<LockedFile>> {
    let parent = match resolved_parent_nofollow(path) {
        Ok(parent) => parent,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let fd = match openat(
        &parent.directory,
        &parent.name,
        OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK,
        Mode::empty(),
    ) {
        Ok(fd) => fd,
        Err(rustix::io::Errno::NOENT) => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    verify_private_fd(&fd, EntryKind::File, PRIVATE_FILE_MODE)?;
    match flock(&fd, FlockOperation::NonBlockingLockExclusive) {
        Ok(()) => Ok(Some(LockedFile { file: fd.into() })),
        Err(rustix::io::Errno::WOULDBLOCK) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

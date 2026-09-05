use super::*;

pub fn create_directory_noreplace(path: &Path, mode: u32) -> io::Result<()> {
    create_directory_noreplace_resolved(&resolved_parent(path)?, mode)
}

pub fn create_directory_noreplace_resolved(target: &ResolvedParent, mode: u32) -> io::Result<()> {
    mkdirat(&target.directory, &target.name, Mode::from_raw_mode(mode)).map_err(io::Error::from)?;
    fsync(&target.directory).map_err(io::Error::from)
}

pub fn create_file_noreplace(path: &Path, mode: u32) -> io::Result<File> {
    create_file_noreplace_resolved(&resolved_parent(path)?, mode)
}

pub fn create_file_noreplace_resolved(target: &ResolvedParent, mode: u32) -> io::Result<File> {
    let fd = openat(
        &target.directory,
        &target.name,
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::CLOEXEC | OFlags::NOFOLLOW,
        Mode::from_raw_mode(mode),
    )
    .map_err(io::Error::from)?;
    fsync(&target.directory).map_err(io::Error::from)?;
    Ok(fd.into())
}

pub fn create_symlink_noreplace(path: &Path, target: &Path) -> io::Result<()> {
    let parent = resolved_parent(path)?;
    symlinkat(target, &parent.directory, &parent.name).map_err(io::Error::from)?;
    fsync(&parent.directory).map_err(io::Error::from)
}

pub fn remove_path(path: &Path) -> io::Result<()> {
    let parent = resolved_parent(path)?;
    remove_entry_from(&parent.directory, &parent.name)?;
    fsync(&parent.directory).map_err(io::Error::from)
}

pub fn remove_empty_directory(path: &Path) -> io::Result<()> {
    let parent = resolved_parent(path)?;
    match stat_in(&parent.directory, &parent.name) {
        Ok(stat) if stat.kind == EntryKind::Directory => {}
        Ok(_) => {
            return Err(invalid_data(format!(
                "{} is not a directory",
                path.display()
            )));
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    }
    unlinkat(&parent.directory, &parent.name, AtFlags::REMOVEDIR).map_err(io::Error::from)?;
    fsync(&parent.directory).map_err(io::Error::from)
}

pub fn remove_nondirectory(path: &Path) -> io::Result<()> {
    let parent = resolved_parent(path)?;
    match stat_in(&parent.directory, &parent.name) {
        Ok(stat) if stat.kind != EntryKind::Directory => {}
        Ok(_) => return Err(invalid_data(format!("{} is a directory", path.display()))),
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    }
    unlinkat(&parent.directory, &parent.name, AtFlags::empty()).map_err(io::Error::from)?;
    fsync(&parent.directory).map_err(io::Error::from)
}

pub(super) fn remove_entry_from(parent: &OwnedFd, name: &OsStr) -> io::Result<()> {
    let stat = match stat_in(parent, name) {
        Ok(stat) => stat,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    if stat.kind == EntryKind::Directory {
        let fd = openat(
            parent,
            name,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC | OFlags::NOFOLLOW,
            Mode::empty(),
        )
        .map_err(io::Error::from)?;
        let opened = stat_value(fstat(&fd).map_err(io::Error::from)?);
        if opened.dev != stat.dev || opened.ino != stat.ino {
            return Err(invalid_data("directory changed while it was being removed"));
        }
        for child in directory_names_from(&fd)? {
            remove_entry_from(&fd, &child)?;
        }
        let current = stat_in(parent, name)?;
        if current.dev != stat.dev || current.ino != stat.ino {
            return Err(invalid_data("directory changed while it was being removed"));
        }
        unlinkat(parent, name, AtFlags::REMOVEDIR).map_err(io::Error::from)
    } else {
        let current = stat_in(parent, name)?;
        if current.dev != stat.dev || current.ino != stat.ino {
            return Err(invalid_data("file changed while it was being removed"));
        }
        unlinkat(parent, name, AtFlags::empty()).map_err(io::Error::from)
    }
}

use std::path::{Path, PathBuf};

pub const SYSFS_BLOCK_DIR: &str = "/sys/class/block";
const SECTOR_BYTES: u64 = 512;
const HOTPLUG_BUSES: [&str; 6] = [
    "usb",
    "mmc",
    "memstick",
    "firewire",
    "ieee1394",
    "thunderbolt",
];
const KNOWN_BUSES: [&str; 6] = ["nvme", "scsi", "ata", "virtio", "mmc", "usb"];
const VIRTUAL_PREFIXES: [&str; 4] = ["ram", "zram", "dm-", "md"];

#[derive(Clone, Debug)]
pub struct BlockDevice {
    pub name: String,
    pub device_number: String,
    pub size: u64,
    pub read_only: bool,
    pub removable: bool,
    pub partition: bool,
    pub bus: String,
    pub hotplug: bool,
    pub backing: Option<String>,
    pub loop_backing: Option<String>,
}

fn attribute(directory: &Path, name: &str) -> Option<String> {
    std::fs::read_to_string(directory.join(name))
        .ok()
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

fn flag(directory: &Path, name: &str) -> bool {
    matches!(attribute(directory, name).as_deref(), Some("1"))
}

fn first_child(directory: &Path, name: &str) -> Option<String> {
    std::fs::read_dir(directory.join(name))
        .ok()?
        .flatten()
        .next()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
}

fn has_partitions(directory: &Path, name: &str) -> bool {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return false;
    };
    entries.flatten().any(|entry| {
        entry.file_name().to_string_lossy().starts_with(name)
            && entry.path().join("partition").is_file()
    })
}

pub fn bus_of(name: &str) -> (String, bool) {
    let Ok(mut node) = std::fs::canonicalize(Path::new(SYSFS_BLOCK_DIR).join(name)) else {
        return ("unknown".to_string(), false);
    };
    let mut chain: Vec<String> = Vec::new();
    while node.parent().is_some() && node != Path::new("/sys") && node != Path::new("/") {
        if let Ok(subsystem) = std::fs::canonicalize(node.join("subsystem"))
            && let Some(subsystem) = subsystem.file_name()
        {
            let subsystem = subsystem.to_string_lossy().into_owned();
            if HOTPLUG_BUSES.contains(&subsystem.as_str()) {
                return (subsystem, true);
            }
            chain.push(subsystem);
        }
        let Some(parent) = node.parent() else { break };
        node = parent.to_path_buf();
    }
    let known = KNOWN_BUSES
        .iter()
        .find(|bus| chain.iter().any(|entry| entry == *bus))
        .map(|bus| (*bus).to_string());
    (
        known
            .or_else(|| chain.last().cloned())
            .unwrap_or_else(|| "unknown".to_string()),
        false,
    )
}

fn removable(directory: &Path) -> bool {
    if !directory.join("partition").is_file() {
        return flag(directory, "removable");
    }
    std::fs::canonicalize(directory)
        .ok()
        .and_then(|resolved| resolved.parent().map(Path::to_path_buf))
        .is_some_and(|parent| flag(&parent, "removable"))
}

pub fn read(name: &str) -> Option<BlockDevice> {
    let directory = PathBuf::from(SYSFS_BLOCK_DIR).join(name);
    let device_number = attribute(&directory, "dev")?;
    let size = attribute(&directory, "size")?.parse::<u64>().ok()? * SECTOR_BYTES;
    let backing = first_child(&directory, "slaves");
    let loop_backing = attribute(&directory.join("loop"), "backing_file");
    if name.starts_with("loop") && loop_backing.is_none() {
        return None;
    }
    let (bus, hotplug) = bus_of(backing.as_deref().unwrap_or(name));
    Some(BlockDevice {
        name: name.to_string(),
        device_number,
        size,
        read_only: flag(&directory, "ro"),
        removable: removable(&directory),
        partition: directory.join("partition").is_file(),
        bus,
        hotplug,
        backing,
        loop_backing,
    })
}

pub fn list() -> Vec<BlockDevice> {
    let Ok(entries) = std::fs::read_dir(SYSFS_BLOCK_DIR) else {
        return Vec::new();
    };
    let mut devices: Vec<BlockDevice> = entries
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| !name.starts_with("ram") && !name.starts_with("zram"))
        .filter(|name| !has_partitions(&PathBuf::from(SYSFS_BLOCK_DIR).join(name), name))
        .filter_map(|name| read(&name))
        .filter(|device| device.size > 0)
        .collect();
    devices.sort_by(|left, right| left.name.cmp(&right.name));
    devices
}

impl BlockDevice {
    pub fn path(&self) -> String {
        format!("/dev/{}", self.name)
    }

    pub fn virtual_device(&self) -> bool {
        VIRTUAL_PREFIXES
            .iter()
            .any(|prefix| self.name.starts_with(prefix))
    }
}

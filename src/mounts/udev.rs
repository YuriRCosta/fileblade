use std::collections::HashMap;

pub const UDEV_DATABASE_DIR: &str = "/run/udev/data";

#[derive(Default)]
pub struct UdevProperties {
    values: HashMap<String, String>,
}

pub fn unescape(value: &str) -> String {
    if !value.contains("\\x") {
        return value.to_string();
    }
    let bytes = value.as_bytes();
    let mut decoded: Vec<u8> = Vec::with_capacity(value.len());
    let mut index = 0;
    while index < bytes.len() {
        let escape = bytes
            .get(index..index + 4)
            .filter(|window| window[0] == b'\\' && window[1] == b'x')
            .and_then(|window| std::str::from_utf8(&window[2..]).ok())
            .and_then(|digits| u8::from_str_radix(digits, 16).ok());
        match escape {
            Some(byte) => {
                decoded.push(byte);
                index += 4;
            }
            None => {
                decoded.push(bytes[index]);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

impl UdevProperties {
    pub fn read(device_number: &str) -> Self {
        let path = format!("{UDEV_DATABASE_DIR}/b{device_number}");
        match std::fs::read_to_string(path) {
            Ok(text) => Self::from_text(&text),
            Err(_) => Self::default(),
        }
    }

    pub fn from_text(text: &str) -> Self {
        let mut values = HashMap::new();
        for line in text.lines() {
            let Some(entry) = line.strip_prefix("E:") else {
                continue;
            };
            let Some((key, value)) = entry.split_once('=') else {
                continue;
            };
            values.insert(key.to_string(), unescape(value));
        }
        Self { values }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.values
            .get(key)
            .map(String::as_str)
            .filter(|value| !value.is_empty())
    }

    pub fn flag(&self, key: &str) -> bool {
        matches!(self.get(key), Some("1"))
    }

    pub fn filesystem(&self) -> Option<&str> {
        self.get("ID_FS_TYPE")
    }

    pub fn label(&self) -> Option<&str> {
        self.get("ID_FS_LABEL")
    }

    pub fn bus(&self) -> Option<&str> {
        self.get("ID_BUS")
    }

    pub fn model(&self) -> Option<&str> {
        self.get("ID_MODEL")
    }

    pub fn ignored(&self) -> bool {
        self.flag("UDISKS_IGNORE") || self.flag("UDISKS_IGNORE_1")
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

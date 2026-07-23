use std::path::PathBuf;

use crate::defaults::APP_ID;

fn data_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(APP_ID)
}

fn ensure_dir(path: &std::path::Path) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
}

pub fn save_json(path: &str, data: &impl serde::Serialize) {
    let full_path = data_dir().join(path);
    ensure_dir(&full_path);
    if let Ok(content) = serde_json::to_string_pretty(data) {
        let _ = std::fs::write(&full_path, content);
    }
}

pub fn load_json<T: serde::de::DeserializeOwned + Default>(path: &str) -> T {
    let full_path = data_dir().join(path);
    match std::fs::read_to_string(&full_path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => T::default(),
    }
}

pub fn remove_file(path: &str) {
    let full_path = data_dir().join(path);
    let _ = std::fs::remove_file(full_path);
}

/// Load a specific section from a shared JSON object file
pub fn load_section<T: serde::de::DeserializeOwned + Default>(path: &str, section: &str) -> T {
    let full_path = data_dir().join(path);
    match std::fs::read_to_string(&full_path) {
        Ok(content) => {
            if let Ok(obj) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(val) = obj.get(section) {
                    return serde_json::from_value(val.clone()).unwrap_or_default();
                }
            }
            T::default()
        }
        Err(_) => T::default(),
    }
}

/// Save a specific section into a shared JSON object file
pub fn save_section<T: serde::Serialize>(path: &str, section: &str, data: &T) {
    let full_path = data_dir().join(path);
    ensure_dir(&full_path);
    let mut obj: serde_json::Value = match std::fs::read_to_string(&full_path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or(serde_json::json!({})),
        Err(_) => serde_json::json!({}),
    };
    obj[section] = serde_json::to_value(data).unwrap();
    if let Ok(content) = serde_json::to_string_pretty(&obj) {
        let _ = std::fs::write(&full_path, content);
    }
}

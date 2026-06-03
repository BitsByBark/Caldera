use std::fs;
use std::path::PathBuf;
use std::sync::{OnceLock, RwLock};

use serde_json::{Map, Value};

static WORKING_DIR_OVERRIDE: OnceLock<RwLock<Option<PathBuf>>> = OnceLock::new();

fn working_dir_cell() -> &'static RwLock<Option<PathBuf>> {
    WORKING_DIR_OVERRIDE.get_or_init(|| RwLock::new(None))
}

pub fn set_working_directory(path: Option<String>) -> Result<String, String> {
    let next = match path {
        Some(raw) if !raw.trim().is_empty() => {
            let pb = PathBuf::from(raw.trim());
            if !pb.exists() || !pb.is_dir() {
                return Err("Working directory must be an existing directory".to_string());
            }
            Some(pb)
        }
        _ => None,
    };

    let cell = working_dir_cell();
    {
        let mut guard = cell
            .write()
            .map_err(|_| "Working directory lock poisoned".to_string())?;
        *guard = next;
    }

    Ok(base_config_dir().to_string_lossy().to_string())
}

pub fn base_config_dir() -> PathBuf {
    let cell = working_dir_cell();
    if let Ok(guard) = cell.read() {
        if let Some(p) = guard.as_ref() {
            return p.clone();
        }
    }

    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("caldera")
}

fn settings_values_path() -> PathBuf {
    base_config_dir().join("settings.values.json")
}

fn settings_schema_path() -> PathBuf {
    base_config_dir().join("settings.brk")
}

fn default_settings_schema() -> &'static str {
    include_str!("../../defaults/settings.brk")
}

pub fn get_settings_schema() -> Result<String, String> {
    let path = settings_schema_path();
    if !path.exists() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed creating runtime dir {}: {}", parent.display(), e))?;
        }
        fs::write(&path, default_settings_schema())
            .map_err(|e| format!("Failed writing settings schema {}: {}", path.display(), e))?;
    }
    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("Failed reading settings schema {}: {}", path.display(), e))?;
    crate::filehandler::parser::parse_settings_schema(&raw)
        .map_err(|e| format!("Failed parsing settings schema {}: {}", path.display(), e))?;
    Ok(raw)
}

pub fn get_settings_values() -> Result<Value, String> {
    let path = settings_values_path();
    if !path.exists() {
        return Ok(Value::Object(Map::new()));
    }
    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("Failed reading settings values {}: {}", path.display(), e))?;
    if raw.trim().is_empty() {
        return Ok(Value::Object(Map::new()));
    }
    serde_json::from_str::<Value>(&raw)
        .map_err(|e| format!("Invalid settings values JSON {}: {}", path.display(), e))
}

pub fn save_settings_values(values: Value) -> Result<(), String> {
    if !values.is_object() {
        return Err("Settings values payload must be an object".to_string());
    }
    let path = settings_values_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed creating runtime dir {}: {}", parent.display(), e))?;
    }
    let body = serde_json::to_string_pretty(&values)
        .map_err(|e| format!("Failed serializing settings values: {}", e))?;
    fs::write(&path, body)
        .map_err(|e| format!("Failed writing settings values {}: {}", path.display(), e))
}

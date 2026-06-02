use std::path::PathBuf;
use std::{fs, path::Path};

use crate::GameConfig;
use serde::{Deserialize, Serialize};
use serde_json::Value;

fn base_config_dir() -> PathBuf {
    crate::runtime::base_config_dir()
}

fn game_config_path(game_id: &str) -> PathBuf {
    base_config_dir()
        .join("games")
        .join(format!("{}.toml", game_id))
}

pub fn get_game_config_stub(game_id: String) -> GameConfig {
    let _path = game_config_path(&game_id);
    GameConfig {
        game_id: game_id.clone(),
        name: format!("Game {}", game_id),
        mod_directory: "~/CALDERA/mods".to_string(),
        deployer: None,
        active_profile: None,
        profiles: Vec::new(),
    }
}

pub fn save_game_config_stub(game_id: String, config: GameConfig) {
    let _path = game_config_path(&game_id);
    let _serialized = toml::to_string(&config).unwrap_or_default();
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModListing {
    pub mod_id: String,
    pub name: String,
    pub status: String,
    pub source_path: Option<String>,
    #[serde(default)]
    pub deployable: bool,
    pub deployer_reason: Option<String>,
    pub added_at: Option<String>,
    pub progress: Option<f32>,
    pub speed: Option<String>,
    pub version: Option<String>,
    pub author: Option<String>,
    pub description: Option<String>,
    pub summary: Option<String>,
    pub source: Option<String>,
    pub source_url: Option<String>,
    pub nexus_mod_id: Option<f64>,
    pub nexus_file_id: Option<f64>,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub file_size: Option<f64>,
    pub file_count: Option<f64>,
    #[serde(default)]
    pub file_types: Vec<String>,
    pub user_notes: Option<String>,
    pub favorite: Option<bool>,
    #[serde(default)]
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileModRow {
    pub mod_id: String,
    pub name: String,
    pub date_added: String,
    pub status: String,
    pub toggleable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ManifestView {
    files: Vec<ManifestFileView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ManifestFileView {
    target: String,
}

fn slugify_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut last_dash = false;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

fn nexus_domain_for_name(name: &str) -> String {
    name.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect()
}

fn value_string(value: &Value, path: &[&str]) -> Option<String> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_str().map(str::to_string)
}

fn value_number(value: &Value, path: &[&str]) -> Option<f64> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_f64()
}

fn selected_file<'a>(metadata: &'a Value, file_id: &str) -> Option<&'a Value> {
    metadata
        .get("files")
        .and_then(|files| files.get("files"))
        .and_then(Value::as_array)
        .and_then(|files| {
            files.iter().find(|file| {
                file.get("file_id")
                    .and_then(Value::as_i64)
                    .map(|id| id.to_string() == file_id)
                    .unwrap_or(false)
            })
        })
}

fn collect_queued_nexus_listings(game_name: &str, out: &mut Vec<ModListing>) -> Result<(), String> {
    let domain = nexus_domain_for_name(game_name);
    if domain.is_empty() {
        return Ok(());
    }

    let root = base_config_dir().join("cache").join(&domain);
    if !root.exists() || !root.is_dir() {
        return Ok(());
    }

    let entries = fs::read_dir(&root)
        .map_err(|e| format!("Failed reading Nexus cache dir {}: {}", root.display(), e))?;
    for entry in entries.flatten() {
        let meta_path = entry.path().join("meta.json");
        if !meta_path.is_file() {
            continue;
        }

        let raw = match fs::read_to_string(&meta_path) {
            Ok(raw) => raw,
            Err(_) => continue,
        };
        let metadata = match serde_json::from_str::<Value>(&raw) {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };

        let mod_id = value_number(&metadata, &["mod", "mod_id"])
            .map(|id| id.trunc().to_string())
            .or_else(|| entry.file_name().to_str().map(str::to_string))
            .unwrap_or_else(|| "unknown".to_string());
        let file_id = value_string(&metadata, &["download", "file_id"]);
        let file = file_id
            .as_deref()
            .and_then(|id| selected_file(&metadata, id));
        let name = file
            .and_then(|f| value_string(f, &["file_name"]))
            .or_else(|| file.and_then(|f| value_string(f, &["name"])))
            .or_else(|| value_string(&metadata, &["mod", "name"]))
            .unwrap_or_else(|| format!("Nexus mod {}", mod_id));

        out.push(ModListing {
            mod_id: format!(
                "nexus-{}-{}-{}",
                domain,
                mod_id,
                file_id.clone().unwrap_or_else(|| "unknown".to_string())
            ),
            name,
            status: "downloading".to_string(),
            source_path: Some(meta_path.to_string_lossy().to_string()),
            deployable: false,
            deployer_reason: Some("Download still in progress".to_string()),
            added_at: value_string(&metadata, &["download", "queued_at"]),
            progress: Some(0.0),
            speed: None,
            version: file
                .and_then(|f| value_string(f, &["version"]))
                .or_else(|| value_string(&metadata, &["mod", "version"])),
            author: value_string(&metadata, &["mod", "author"])
                .or_else(|| value_string(&metadata, &["mod", "uploaded_by"])),
            description: file
                .and_then(|f| value_string(f, &["description"]))
                .or_else(|| value_string(&metadata, &["mod", "description"])),
            summary: value_string(&metadata, &["mod", "summary"]),
            source: Some("nexus".to_string()),
            source_url: value_string(&metadata, &["download", "url"]),
            nexus_mod_id: value_number(&metadata, &["mod", "mod_id"]),
            nexus_file_id: file_id.and_then(|id| id.parse::<f64>().ok()),
            categories: Vec::new(),
            tags: Vec::new(),
            file_size: file.and_then(|f| value_number(f, &["size_in_bytes"])),
            file_count: None,
            file_types: Vec::new(),
            user_notes: None,
            favorite: None,
            files: Vec::new(),
        });
    }

    Ok(())
}

fn listing_roots(game_name: &str, app_id: &str) -> Vec<PathBuf> {
    let key = format!("{}-{}", slugify_name(game_name), app_id);
    vec![
        // Canonical game folder
        base_config_dir().join("downloads").join(key),
    ]
}

fn maybe_collect_from_dir(dir: &Path, out: &mut Vec<ModListing>) -> Result<(), String> {
    if !dir.exists() || !dir.is_dir() {
        return Ok(());
    }
    let entries = fs::read_dir(dir)
        .map_err(|e| format!("Failed reading listings dir {}: {}", dir.display(), e))?;
    for entry in entries.flatten() {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        let is_json = p
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("json"))
            .unwrap_or(false);
        if is_json {
            let raw = match fs::read_to_string(&p) {
                Ok(s) => s,
                Err(_) => continue,
            };
            if let Ok(mut parsed) = serde_json::from_str::<ModListing>(&raw) {
                if parsed.source_path.is_none() {
                    parsed.source_path = Some(p.to_string_lossy().to_string());
                }
                if parsed.files.is_empty() {
                    if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                        parsed.files.push(name.to_string());
                    }
                }
                out.push(parsed);
                continue;
            }
        }

        // Fallback: treat any plain file as a listing row.
        let file_name = p
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown-file")
            .to_string();
        let mod_id = p
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown-mod")
            .to_string();
        out.push(ModListing {
            mod_id,
            name: file_name.clone(),
            status: "downloaded".to_string(),
            source_path: Some(p.to_string_lossy().to_string()),
            deployable: false,
            deployer_reason: None,
            added_at: None,
            progress: Some(1.0),
            speed: Some("0 KB/S".to_string()),
            version: None,
            author: None,
            description: None,
            summary: None,
            source: Some("local".to_string()),
            source_url: None,
            nexus_mod_id: None,
            nexus_file_id: None,
            categories: Vec::new(),
            tags: Vec::new(),
            file_size: None,
            file_count: None,
            file_types: Vec::new(),
            user_notes: None,
            favorite: None,
            files: vec![file_name],
        });
    }
    Ok(())
}

pub fn get_raw_modlist_listings(app_id: String) -> Result<Vec<ModListing>, String> {
    let meta = crate::deployer::read_game_meta(&app_id)?;
    let mut out = Vec::new();
    for root in listing_roots(&meta.name, &app_id) {
        maybe_collect_from_dir(&root, &mut out)?;
    }
    collect_queued_nexus_listings(&meta.name, &mut out)?;
    out.sort_by(|a, b| b.added_at.cmp(&a.added_at));
    Ok(out)
}

pub fn get_modlist_listings(app_id: String) -> Result<Vec<ModListing>, String> {
    get_raw_modlist_listings(app_id)
}

fn manifest_status_for_mod(app_id: &str, mod_id: &str) -> Result<(String, bool), String> {
    let manifest_path = base_config_dir()
        .join("cache")
        .join(app_id)
        .join("mods")
        .join(mod_id)
        .join("manifest.json");

    if !manifest_path.exists() {
        return Ok(("UNKNOWN".to_string(), false));
    }

    let raw = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed reading manifest {}: {}", manifest_path.display(), e))?;
    let manifest: ManifestView = serde_json::from_str(&raw)
        .map_err(|e| format!("Invalid manifest JSON {}: {}", manifest_path.display(), e))?;

    if manifest.files.is_empty() {
        return Ok(("UNKNOWN".to_string(), false));
    }

    let mut all_enabled = true;
    let mut all_disabled = true;

    for f in manifest.files {
        let enabled_path = PathBuf::from(&f.target);
        let disabled_path = PathBuf::from(format!("{}.disabled", f.target));

        let enabled_exists = enabled_path.exists();
        let disabled_exists = disabled_path.exists();

        if !enabled_exists {
            all_enabled = false;
        }
        if !disabled_exists {
            all_disabled = false;
        }
    }

    if all_enabled {
        Ok(("ENABLED".to_string(), true))
    } else if all_disabled {
        Ok(("DISABLED".to_string(), true))
    } else {
        Ok(("UNKNOWN".to_string(), false))
    }
}

pub fn get_profile_modlist(app_id: String) -> Result<Vec<ProfileModRow>, String> {
    let profile_path = crate::profile_runtime::profile_path(&app_id)?;
    if !profile_path.exists() {
        return Ok(Vec::new());
    }

    let raw = fs::read_to_string(&profile_path)
        .map_err(|e| format!("Failed reading profile {}: {}", profile_path.display(), e))?;
    let profile = crate::profile_format::parse_profile(&raw)
        .map_err(|e| format!("Failed parsing profile {}: {}", profile_path.display(), e))?;

    let mut rows = Vec::with_capacity(profile.modlist.len());
    for entry in profile.modlist {
        let (status, toggleable) = manifest_status_for_mod(&app_id, &entry.id)?;
        rows.push(ProfileModRow {
            mod_id: entry.id,
            name: entry.name,
            date_added: "--".to_string(),
            status,
            toggleable,
        });
    }
    Ok(rows)
}

pub fn profile_contains_mod(app_id: &str, mod_id: &str) -> Result<bool, String> {
    let profile_path = crate::profile_runtime::profile_path(app_id)?;
    if !profile_path.exists() {
        return Ok(false);
    }
    let raw = fs::read_to_string(&profile_path)
        .map_err(|e| format!("Failed reading profile {}: {}", profile_path.display(), e))?;
    let profile = crate::profile_format::parse_profile(&raw)
        .map_err(|e| format!("Failed parsing profile {}: {}", profile_path.display(), e))?;
    Ok(profile.modlist.iter().any(|m| m.id == mod_id))
}

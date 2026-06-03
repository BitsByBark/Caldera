use std::path::PathBuf;
use std::{fs, path::Path};

use crate::{AppError, GameConfig, WithPath};
use serde::{Deserialize, Serialize};

pub fn get_game_config(game_id: String) -> GameConfig {
    let path = crate::filehandler::runtime_reader::game_config_path(&game_id);
    if path.exists() {
        if let Ok(raw) = fs::read_to_string(&path) {
            if let Ok(config) = toml::from_str::<GameConfig>(&raw) {
                return config;
            }
        }
    }
    GameConfig {
        game_id: game_id.clone(),
        name: game_id.clone(),
        mod_directory: String::new(),
        game_domain: None,
        deployer: None,
        active_profile: Some(crate::filehandler::runtime_reader::DEFAULT_PROFILE.to_string()),
        profiles: vec![crate::filehandler::runtime_reader::DEFAULT_PROFILE.to_string()],
    }
}

pub fn save_game_config(game_id: String, config: GameConfig) -> Result<(), AppError> {
    let path = crate::filehandler::runtime_reader::game_config_path(&game_id);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_path(parent)?;
    }
    let body = toml::to_string(&config).map_err(AppError::TomlSerialize)?;
    fs::write(&path, body).with_path(&path)
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

fn listing_roots(game_name: &str, app_id: &str) -> Vec<PathBuf> {
    let _ = game_name;
    vec![crate::filehandler::runtime_reader::mods_dir(app_id)]
}

fn is_listing_metadata_path(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            e.eq_ignore_ascii_case("json")
                || e.eq_ignore_ascii_case("toml")
                || path.file_name().and_then(|n| n.to_str()) == Some("meta.toml")
        })
        .unwrap_or(false)
}

fn write_listing_file(path: &Path, listing: &ModListing) -> Result<(), AppError> {
    let body = if path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("json"))
        .unwrap_or(false)
    {
        serde_json::to_string_pretty(listing).map_err(AppError::Json)?
    } else {
        toml::to_string_pretty(listing).map_err(AppError::TomlSerialize)?
    };
    fs::write(path, body).with_path(path)
}

fn source_path_matches_archive(source: &str, archive: &Path) -> bool {
    let listed = PathBuf::from(source);
    if listed == archive {
        return true;
    }
    if let (Ok(a), Ok(b)) = (listed.canonicalize(), archive.canonicalize()) {
        return a == b;
    }
    false
}

fn find_listing_path_for_archive(archive: &Path) -> Option<PathBuf> {
    let parent = archive.parent()?;

    if parent
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.eq_ignore_ascii_case("files"))
    {
        if let Some(mod_root) = parent.parent() {
            let meta = mod_root.join("meta.toml");
            if meta.is_file() {
                return Some(meta);
            }
        }
    }

    let archive_str = archive.to_string_lossy();
    let entries = fs::read_dir(parent).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() || !is_listing_metadata_path(&path) {
            continue;
        }
        let raw = fs::read_to_string(&path).ok()?;
        let listing = parse_listing_file(&raw, &path)?;
        if listing
            .source_path
            .as_deref()
            .is_some_and(|s| source_path_matches_archive(s, archive) || s == archive_str)
        {
            return Some(path);
        }
    }

    None
}

/// Point listing metadata at extracted files and drop the archive from `files`.
pub fn update_listing_after_uncompress(
    archive_path: &Path,
    extract_dir: &Path,
    extracted_files: &[PathBuf],
) -> Result<(), AppError> {
    let Some(listing_path) = find_listing_path_for_archive(archive_path) else {
        return Ok(());
    };

    let raw = fs::read_to_string(&listing_path).with_path(&listing_path)?;
    let mut listing = parse_listing_file(&raw, &listing_path).ok_or_else(|| {
        AppError::other(format!(
            "Failed parsing listing metadata {}",
            listing_path.display()
        ))
    })?;

    let archive_str = archive_path.to_string_lossy();
    let under_mod_files = listing_path
        .parent()
        .map(|mod_root| {
            let files_dir = mod_root.join("files");
            archive_path.starts_with(&files_dir)
        })
        .unwrap_or(false);
    let matches = listing
        .source_path
        .as_deref()
        .is_some_and(|s| source_path_matches_archive(s, archive_path) || s == archive_str)
        || under_mod_files;
    if !matches {
        return Ok(());
    }

    let display_name = archive_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("extracted")
        .to_string();

    listing.source_path = Some(extract_dir.to_string_lossy().to_string());
    listing.name = display_name;
    listing.files = extracted_files
        .iter()
        .filter_map(|p| p.file_name().and_then(|n| n.to_str().map(str::to_string)))
        .collect();
    if listing.files.is_empty() {
        listing.file_count = Some(0.0);
    } else {
        listing.file_count = Some(listing.files.len() as f64);
    }

    write_listing_file(&listing_path, &listing)
}

fn parse_listing_file(raw: &str, path: &Path) -> Option<ModListing> {
    if path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("json"))
        .unwrap_or(false)
    {
        serde_json::from_str::<ModListing>(raw).ok()
    } else {
        toml::from_str::<ModListing>(raw).ok()
    }
}

fn files_in_dir(dir: &Path) -> Vec<PathBuf> {
    fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|p| p.is_file())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn listing_for_file(base: &ModListing, mod_id: &str, file_path: &Path) -> ModListing {
    let file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown-file")
        .to_string();
    let mut listing = base.clone();
    listing.mod_id = format!(
        "{}:{}",
        mod_id,
        crate::filehandler::runtime_reader::safe_runtime_name(&file_name, "file")
    );
    listing.name = file_name.clone();
    listing.status = "downloaded".to_string();
    listing.source_path = Some(file_path.to_string_lossy().to_string());
    listing.progress = Some(1.0);
    listing.speed = Some("0 KB/S".to_string());
    listing.file_size = fs::metadata(file_path).ok().map(|m| m.len() as f64);
    listing.file_count = Some(1.0);
    listing.files = vec![file_name];
    listing
}

fn fallback_listing_for_file(mod_id: &str, file_path: &Path) -> ModListing {
    let file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown-file")
        .to_string();
    ModListing {
        mod_id: format!(
            "{}:{}",
            mod_id,
            crate::filehandler::runtime_reader::safe_runtime_name(&file_name, "file")
        ),
        name: file_name.clone(),
        status: "downloaded".to_string(),
        source_path: Some(file_path.to_string_lossy().to_string()),
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
        file_size: fs::metadata(file_path).ok().map(|m| m.len() as f64),
        file_count: Some(1.0),
        file_types: Vec::new(),
        user_notes: None,
        favorite: None,
        files: vec![file_name],
    }
}

fn maybe_collect_from_dir(dir: &Path, out: &mut Vec<ModListing>) -> Result<(), AppError> {
    if !dir.exists() || !dir.is_dir() {
        return Ok(());
    }
    let entries = fs::read_dir(dir).with_path(dir)?;
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            let mod_id = p
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown-mod")
                .to_string();
            let meta_path = p.join("meta.toml");
            let actual_files = files_in_dir(&p.join("files"));
            if meta_path.is_file() {
                if let Ok(raw) = fs::read_to_string(&meta_path) {
                    if let Some(mut parsed) = parse_listing_file(&raw, &meta_path) {
                        if !actual_files.is_empty() {
                            for file_path in &actual_files {
                                out.push(listing_for_file(&parsed, &mod_id, file_path));
                            }
                            continue;
                        }
                        if parsed.source_path.is_none() {
                            parsed.source_path =
                                Some(p.join("files").to_string_lossy().to_string());
                        }
                        if parsed.files.is_empty() {
                            if let Ok(files) = fs::read_dir(p.join("files")) {
                                parsed.files = files
                                    .flatten()
                                    .filter_map(|entry| {
                                        entry.file_name().to_str().map(str::to_string)
                                    })
                                    .collect();
                            }
                        }
                        out.push(parsed);
                        continue;
                    }
                }
            }

            if !actual_files.is_empty() {
                for file_path in &actual_files {
                    out.push(fallback_listing_for_file(&mod_id, file_path));
                }
                continue;
            }

            out.push(ModListing {
                mod_id: mod_id.clone(),
                name: mod_id,
                status: "downloaded".to_string(),
                source_path: Some(p.join("files").to_string_lossy().to_string()),
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
                file_count: Some(0.0),
                file_types: Vec::new(),
                user_notes: None,
                favorite: None,
                files: Vec::new(),
            });
            continue;
        }
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
            if let Some(mut parsed) = parse_listing_file(&raw, &p) {
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

pub fn get_raw_modlist_listings(app_id: String) -> Result<Vec<ModListing>, AppError> {
    let meta = crate::deployer::read_game_meta(&app_id)?;
    let mut out = Vec::new();
    for root in listing_roots(&meta.name, &app_id) {
        maybe_collect_from_dir(&root, &mut out)?;
    }
    out.sort_by(|a, b| b.added_at.cmp(&a.added_at));
    Ok(out)
}

pub fn get_modlist_listings(app_id: String) -> Result<Vec<ModListing>, AppError> {
    get_raw_modlist_listings(app_id)
}

fn manifest_status_for_mod(app_id: &str, mod_id: &str) -> Result<(String, bool), AppError> {
    let manifest_path = crate::filehandler::runtime_reader::mod_manifest_path(app_id, mod_id);

    if !manifest_path.exists() {
        return Ok(("UNKNOWN".to_string(), false));
    }

    let raw = fs::read_to_string(&manifest_path).with_path(&manifest_path)?;
    let manifest: ManifestView =
        serde_json::from_str(&raw).map_err(AppError::Json)?;

    if manifest.files.is_empty() {
        return Ok(("UNKNOWN".to_string(), false));
    }

    let mut all_linked = true;
    let mut all_unlinked = true;

    for f in manifest.files {
        let target = PathBuf::from(&f.target);
        let linked = target.symlink_metadata().is_ok();

        if !linked {
            all_linked = false;
        }
        if linked {
            all_unlinked = false;
        }
    }

    if all_linked {
        Ok(("ENABLED".to_string(), true))
    } else if all_unlinked {
        Ok(("DISABLED".to_string(), true))
    } else {
        Ok(("UNKNOWN".to_string(), false))
    }
}

pub fn get_profile_modlist(app_id: String) -> Result<Vec<ProfileModRow>, AppError> {
    let profile_path = crate::filehandler::runtime_reader::default_profile_path(&app_id);
    if !profile_path.exists() {
        return Ok(Vec::new());
    }

    let raw = fs::read_to_string(&profile_path).with_path(&profile_path)?;
    let profile = crate::filehandler::parser::parse_profile(&raw).map_err(|e| {
        AppError::other(format!(
            "Failed parsing profile {}: {}",
            profile_path.display(),
            e
        ))
    })?;

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

pub fn profile_contains_mod(app_id: &str, mod_id: &str) -> Result<bool, AppError> {
    let profile_path = crate::filehandler::runtime_reader::default_profile_path(app_id);
    if !profile_path.exists() {
        return Ok(false);
    }
    let raw = fs::read_to_string(&profile_path).with_path(&profile_path)?;
    let profile = crate::filehandler::parser::parse_profile(&raw).map_err(|e| {
        AppError::other(format!(
            "Failed parsing profile {}: {}",
            profile_path.display(),
            e
        ))
    })?;
    Ok(profile.modlist.iter().any(|m| m.id == mod_id))
}

use std::fs;
use std::path::PathBuf;

use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::config::ModListing;
use crate::deployer::ModManifest;
use crate::filehandler::parser::{
    serialize_profile, CalderaProfile, ConflictRule, ModEntry, ProfileMeta,
};
use crate::{AppError, WithPath};

pub const DEFAULT_PROFILE: &str = "DEFAULT";

pub fn root_dir() -> PathBuf {
    crate::filehandler::runtime::base_config_dir()
}

pub fn registry_path() -> PathBuf {
    root_dir().join("registry.cldr")
}

pub fn library_dir() -> PathBuf {
    root_dir().join("library")
}

fn safe_game_folder_part(value: &str, fallback: &str) -> String {
    let mut out = String::new();
    for ch in value.trim().chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, ' ' | '-' | '_' | '.') {
            out.push(ch);
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    let out = out.trim_matches(|ch| ch == '_' || ch == ' ').to_string();
    if out.is_empty() {
        fallback.to_string()
    } else {
        out
    }
}

pub fn game_folder_name(game_name: &str, app_id: &str) -> String {
    if game_name.trim().is_empty() {
        return safe_game_folder_part(app_id, app_id);
    }
    format!(
        "{}-{}",
        safe_game_folder_part(game_name, app_id),
        safe_game_folder_part(app_id, app_id)
    )
}

fn game_dir_from_library(app_id: &str) -> Option<PathBuf> {
    let library = library_dir();
    let entries = fs::read_dir(&library).ok()?;
    let mut exact = None;
    let mut named = None;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let folder_name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        if folder_name == app_id {
            exact = Some(path.clone());
        }
        let meta_path = path.join("metadata").join("meta.json");
        let Ok(raw) = fs::read_to_string(&meta_path) else { continue; };
        let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&raw) else { continue; };
        if parsed.get("app_id").and_then(|v| v.as_str()) == Some(app_id) {
            if folder_name != app_id {
                named = Some(path);
                break;
            }
            exact = Some(path);
        }
    }

    named.or(exact)
}

fn known_game_name(app_id: &str) -> Option<String> {
    let library = library_dir();
    if let Ok(entries) = fs::read_dir(&library) {
        for entry in entries.flatten() {
            let meta_path = entry.path().join("metadata").join("meta.json");
            let Ok(raw) = fs::read_to_string(&meta_path) else { continue; };
            let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&raw) else { continue; };
            if parsed.get("app_id").and_then(|v| v.as_str()) == Some(app_id) {
                if let Some(name) = parsed.get("name").and_then(|v| v.as_str()) {
                    if !name.trim().is_empty() {
                        return Some(name.to_string());
                    }
                }
            }
        }
    }

    let manual_path = root_dir().join("manual_games.json");
    let raw = fs::read_to_string(&manual_path).ok()?;
    let parsed = serde_json::from_str::<serde_json::Value>(&raw).ok()?;
    for game in parsed.as_array()? {
        if game.get("app_id").and_then(|v| v.as_str()) == Some(app_id) {
            return game
                .get("name")
                .and_then(|v| v.as_str())
                .filter(|name| !name.trim().is_empty())
                .map(str::to_string);
        }
    }
    None
}

pub fn game_dir(app_id: &str) -> PathBuf {
    game_dir_from_library(app_id).unwrap_or_else(|| {
        if let Some(name) = known_game_name(app_id) {
            game_dir_for_name(app_id, &name)
        } else {
            library_dir().join(app_id)
        }
    })
}

pub fn game_dir_for_name(app_id: &str, game_name: &str) -> PathBuf {
    library_dir().join(game_folder_name(game_name, app_id))
}

pub fn metadata_dir(app_id: &str) -> PathBuf {
    game_dir(app_id).join("metadata")
}

pub fn game_meta_path(app_id: &str) -> PathBuf {
    metadata_dir(app_id).join("meta.json")
}

pub fn game_config_path(app_id: &str) -> PathBuf {
    metadata_dir(app_id).join("config.toml")
}

pub fn artwork_dir(app_id: &str) -> PathBuf {
    metadata_dir(app_id).join("artwork")
}

pub fn mods_dir(app_id: &str) -> PathBuf {
    game_dir(app_id).join("mods")
}

pub fn mod_dir(app_id: &str, mod_id: &str) -> PathBuf {
    mods_dir(app_id).join(mod_id)
}

pub fn mod_files_dir(app_id: &str, mod_id: &str) -> PathBuf {
    mod_dir(app_id, mod_id).join("files")
}

pub fn mod_meta_path(app_id: &str, mod_id: &str) -> PathBuf {
    mod_dir(app_id, mod_id).join("meta.toml")
}

pub fn mod_manifest_path(app_id: &str, mod_id: &str) -> PathBuf {
    mod_dir(app_id, mod_id).join("manifest.json")
}

pub fn profiles_dir(app_id: &str) -> PathBuf {
    game_dir(app_id).join("profiles")
}

pub fn collections_dir(app_id: &str) -> PathBuf {
    game_dir(app_id).join("collections")
}

pub fn safe_runtime_name(name: &str, fallback: &str) -> String {
    let mut out = String::new();
    for ch in name.trim().chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
            out.push(ch);
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    let out = out.trim_matches('_').to_string();
    if out.is_empty() {
        fallback.to_string()
    } else {
        out
    }
}

pub fn profile_path(app_id: &str, profile_name: &str) -> PathBuf {
    profiles_dir(app_id).join(format!(
        "{}.cldr",
        safe_runtime_name(profile_name, DEFAULT_PROFILE)
    ))
}

pub fn default_profile_path(app_id: &str) -> PathBuf {
    profile_path(app_id, DEFAULT_PROFILE)
}

pub fn collection_path(app_id: &str, collection_name: &str) -> PathBuf {
    collections_dir(app_id).join(format!(
        "{}.cldr",
        safe_runtime_name(collection_name, "collection")
    ))
}

pub fn ensure_game_dirs(app_id: &str) -> Result<(), AppError> {
    for dir in [
        metadata_dir(app_id),
        artwork_dir(app_id),
        mods_dir(app_id),
        profiles_dir(app_id),
        collections_dir(app_id),
    ] {
        fs::create_dir_all(&dir).with_path(&dir)?;
    }
    Ok(())
}

pub fn ensure_game_dirs_for_name(app_id: &str, game_name: &str) -> Result<(), AppError> {
    let root = game_dir_for_name(app_id, game_name);
    for dir in [
        root.join("metadata"),
        root.join("metadata").join("artwork"),
        root.join("mods"),
        root.join("profiles"),
        root.join("collections"),
    ] {
        fs::create_dir_all(&dir).with_path(&dir)?;
    }
    Ok(())
}

pub fn read_to_string(path: PathBuf) -> Result<String, AppError> {
    let p = path.clone();
    fs::read_to_string(path).with_path(&p)
}

fn now_iso() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

pub fn metadata_game_dir(app_id: &str) -> Result<PathBuf, AppError> {
    Ok(profiles_dir(app_id))
}

fn init_profile(name: &str, deployer: &str) -> CalderaProfile {
    CalderaProfile {
        profile: ProfileMeta {
            name: name.to_string(),
            created: now_iso(),
            modified: now_iso(),
            description: None,
            deployer: deployer.to_string(),
        },
        modlist: Vec::new(),
        conflicts: Vec::<ConflictRule>::new(),
    }
}

pub fn load_or_init_profile(app_id: &str, deployer: &str) -> Result<CalderaProfile, AppError> {
    let path = default_profile_path(app_id);
    if !path.exists() {
        return Ok(init_profile(DEFAULT_PROFILE, deployer));
    }
    let raw = fs::read_to_string(&path).with_path(&path)?;
    match crate::filehandler::parser::parse_profile(&raw) {
        Ok(p) => Ok(p),
        Err(_) => Ok(init_profile(DEFAULT_PROFILE, deployer)),
    }
}

pub fn save_profile(app_id: &str, profile: &CalderaProfile) -> Result<(), AppError> {
    let path = default_profile_path(app_id);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_path(parent)?;
    }
    let body = serialize_profile(profile);
    fs::write(&path, &body).with_path(&path)
}

fn manifest_file_types(manifest: &ModManifest) -> Vec<String> {
    let mut out = Vec::new();
    for f in &manifest.files {
        if let Some(ext) = std::path::Path::new(&f.name)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| format!(".{}", e.to_ascii_lowercase()))
        {
            if !out.contains(&ext) {
                out.push(ext);
            }
        }
    }
    out
}

fn deploy_note(manifest: &ModManifest) -> String {
    let mut lines = Vec::new();
    lines.push(format!("deployed_dir: {}", manifest.target_folder));
    for f in &manifest.files {
        lines.push(format!("deployed_file: {}", f.target));
    }
    lines.join("\n")
}

pub fn upsert_profile_from_deploy(
    app_id: &str,
    deployer_id: &str,
    listing: &ModListing,
    manifest: &ModManifest,
) -> Result<(), AppError> {
    let mut profile = load_or_init_profile(app_id, deployer_id)?;
    profile.profile.modified = now_iso();
    profile.profile.deployer = deployer_id.to_string();

    let mut merged_user_notes = listing.user_notes.clone();
    let deploy_meta_note = deploy_note(manifest);
    merged_user_notes = Some(match merged_user_notes {
        Some(existing) if !existing.trim().is_empty() => {
            format!("{}\n{}", existing, deploy_meta_note)
        }
        _ => deploy_meta_note,
    });

    let mut new_entry = ModEntry {
        id: listing.mod_id.clone(),
        name: if listing.name.trim().is_empty() {
            listing.mod_id.clone()
        } else {
            listing.name.clone()
        },
        version: listing
            .version
            .clone()
            .unwrap_or_else(|| "unknown".to_string()),
        author: listing.author.clone(),
        description: listing.description.clone(),
        summary: listing.summary.clone(),
        source: listing
            .source
            .clone()
            .unwrap_or_else(|| "local".to_string()),
        source_url: listing.source_url.clone(),
        nexus_mod_id: listing.nexus_mod_id,
        nexus_file_id: listing.nexus_file_id,
        categories: listing.categories.clone(),
        tags: listing.tags.clone(),
        file_size: listing.file_size,
        file_count: listing.file_count.or(Some(manifest.files.len() as f64)),
        file_types: if !listing.file_types.is_empty() {
            listing.file_types.clone()
        } else {
            manifest_file_types(manifest)
        },
        user_notes: merged_user_notes,
        favorite: listing.favorite,
    };

    if !["nexus", "gamebanana", "moddb", "itch", "local"].contains(&new_entry.source.as_str()) {
        new_entry.source = "local".to_string();
    }

    if let Some(existing) = profile.modlist.iter_mut().find(|m| m.id == listing.mod_id) {
        let keep_notes = existing.user_notes.clone();
        let keep_fav = existing.favorite;
        *existing = new_entry;
        if existing.user_notes.is_none() {
            existing.user_notes = keep_notes;
        }
        if existing.favorite.is_none() {
            existing.favorite = keep_fav;
        }
    } else {
        profile.modlist.push(new_entry);
    }

    save_profile(app_id, &profile)
}

pub fn list_mod_dirs(app_id: &str) -> Result<Vec<PathBuf>, AppError> {
    let dir = mods_dir(app_id);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(&dir).with_path(&dir)?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.push(path);
        }
    }
    out.sort();
    Ok(out)
}

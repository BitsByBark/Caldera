use std::fs;
use std::path::PathBuf;

use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::config::ModListing;
use crate::deployer::ModManifest;
use crate::profile_format::{serialize_profile, CalderaProfile, ConflictRule, ModEntry, ProfileMeta};

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

fn now_iso() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

pub fn metadata_game_dir(app_id: &str) -> Result<PathBuf, String> {
    let meta = crate::deployer::read_game_meta(app_id)?;
    Ok(crate::runtime::base_config_dir()
        .join("metadata")
        .join(format!("{}-{}", slugify_name(&meta.name), app_id)))
}

pub fn profile_path(app_id: &str) -> Result<PathBuf, String> {
    Ok(metadata_game_dir(app_id)?.join("profile.profile"))
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

pub fn load_or_init_profile(app_id: &str, deployer: &str) -> Result<CalderaProfile, String> {
    let path = profile_path(app_id)?;
    if !path.exists() {
        return Ok(init_profile("profile", deployer));
    }
    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("Failed reading profile {}: {}", path.display(), e))?;
    match crate::profile_format::parse_profile(&raw) {
        Ok(p) => Ok(p),
        Err(_) => Ok(init_profile("profile", deployer)),
    }
}

pub fn save_profile(app_id: &str, profile: &CalderaProfile) -> Result<(), String> {
    let path = profile_path(app_id)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed creating metadata dir {}: {}", parent.display(), e))?;
    }

    let body = serialize_profile(profile);
    fs::write(&path, &body)
        .map_err(|e| format!("Failed writing profile {}: {}", path.display(), e))?;
    Ok(())
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
) -> Result<(), String> {
    let mut profile = load_or_init_profile(app_id, deployer_id)?;
    profile.profile.modified = now_iso();
    profile.profile.deployer = deployer_id.to_string();

    let mut merged_user_notes = listing.user_notes.clone();
    let deploy_meta_note = deploy_note(manifest);
    merged_user_notes = Some(match merged_user_notes {
        Some(existing) if !existing.trim().is_empty() => format!("{}\n{}", existing, deploy_meta_note),
        _ => deploy_meta_note,
    });

    let mut new_entry = ModEntry {
        id: listing.mod_id.clone(),
        name: if listing.name.trim().is_empty() {
            listing.mod_id.clone()
        } else {
            listing.name.clone()
        },
        version: listing.version.clone().unwrap_or_else(|| "unknown".to_string()),
        author: listing.author.clone(),
        description: listing.description.clone(),
        summary: listing.summary.clone(),
        source: listing.source.clone().unwrap_or_else(|| "local".to_string()),
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

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use tar::{Archive, Builder, Header};
use tauri::{AppHandle, Emitter};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::{AppError, WithPath};

const PACK_FORMAT: u32 = 1;
const CALDERA_MIN_VERSION: &str = "0.1.0";
const CALDERA_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub mods_installed: u32,
    pub mods_queued: u32,
    pub mods_failed: u32,
    pub profile_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackManifest {
    pack_format: u32,
    name: String,
    #[serde(rename = "type")]
    pack_type: String,
    created_at: String,
    caldera_min_version: String,
    game: PackGame,
    profile: String,
    mods: Vec<PackMod>,
    include_disabled: bool,
    total_mods: u32,
    total_size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PackGame {
    id: String,
    name: String,
    deployer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PackMod {
    id: String,
    name: String,
    version: String,
    source_url: Option<String>,
    enabled: bool,
    files: Vec<PackFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackFile {
    name: String,
    size: u64,
    checksum: String,
}

#[derive(Debug, Clone, Serialize)]
struct LogEvent {
    message: String,
    level: String,
}

fn emit(app: &AppHandle, level: &str, message: &str) {
    let _ = app.emit(
        "caldera://session-log",
        LogEvent {
            message: message.to_string(),
            level: level.to_string(),
        },
    );
}

fn now_iso() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

fn cache_profile_path(app_id: &str, profile_name: &str) -> PathBuf {
    crate::filehandler::runtime_reader::profile_path(app_id, profile_name)
}

fn mod_dir(app_id: &str, mod_id: &str) -> PathBuf {
    crate::filehandler::runtime_reader::mod_dir(app_id, mod_id)
}

fn safe_name(name: &str) -> String {
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
        "pack".to_string()
    } else {
        out
    }
}

fn append_bytes(
    builder: &mut Builder<Vec<u8>>,
    path: &str,
    bytes: &[u8],
) -> Result<(), AppError> {
    let mut header = Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    builder
        .append_data(&mut header, path, Cursor::new(bytes))
        .map_err(|e| AppError::other(format!("Failed adding {} to archive: {}", path, e)))
}

fn read_profile_text(app_id: &str, profile_name: &str) -> Result<String, AppError> {
    let requested = cache_profile_path(app_id, profile_name);
    if requested.exists() {
        return fs::read_to_string(&requested).with_path(&requested);
    }

    let fallback = crate::filehandler::runtime_reader::default_profile_path(app_id);
    fs::read_to_string(&fallback).with_path(&fallback)
}

fn manifest_enabled(app_id: &str, mod_id: &str) -> Result<bool, AppError> {
    let p = mod_dir(app_id, mod_id).join("manifest.json");
    if !p.exists() {
        return Ok(true);
    }
    let raw = fs::read_to_string(&p).with_path(&p)?;
    let manifest: serde_json::Value = serde_json::from_str(&raw).map_err(AppError::Json)?;
    let files = manifest
        .get("files")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    if files.is_empty() {
        return Ok(true);
    }
    Ok(files.iter().any(|f| {
        f.get("linked")
            .or_else(|| f.get("enabled"))
            .and_then(|v| v.as_bool())
            .unwrap_or(true)
    }))
}

fn mod_meta_toml(entry: &crate::filehandler::parser::ModEntry) -> Result<String, AppError> {
    let mut table = toml::map::Map::new();
    table.insert("id".to_string(), toml::Value::String(entry.id.clone()));
    table.insert("name".to_string(), toml::Value::String(entry.name.clone()));
    table.insert(
        "version".to_string(),
        toml::Value::String(entry.version.clone()),
    );
    table.insert(
        "source".to_string(),
        toml::Value::String(entry.source.clone()),
    );
    if let Some(v) = &entry.source_url {
        table.insert("source_url".to_string(), toml::Value::String(v.clone()));
    }
    if let Some(v) = &entry.author {
        table.insert("author".to_string(), toml::Value::String(v.clone()));
    }
    if let Some(v) = &entry.description {
        table.insert("description".to_string(), toml::Value::String(v.clone()));
    }
    if let Some(v) = &entry.summary {
        table.insert("summary".to_string(), toml::Value::String(v.clone()));
    }
    toml::to_string(&toml::Value::Table(table)).map_err(AppError::TomlSerialize)
}

fn read_or_make_meta(
    app_id: &str,
    entry: &crate::filehandler::parser::ModEntry,
) -> Result<String, AppError> {
    let p = mod_dir(app_id, &entry.id).join("meta.toml");
    if p.exists() {
        return fs::read_to_string(&p).with_path(&p);
    }
    mod_meta_toml(entry)
}

fn read_or_make_manifest(app_id: &str, mod_id: &str) -> Result<String, AppError> {
    let p = mod_dir(app_id, mod_id).join("manifest.json");
    if p.exists() {
        return fs::read_to_string(&p).with_path(&p);
    }
    Ok("{\n  \"deployed\": false,\n  \"files\": []\n}".to_string())
}

fn bundled_files(app_id: &str, mod_id: &str) -> Result<Vec<PathBuf>, AppError> {
    let dir = mod_dir(app_id, mod_id).join("files");
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    let entries = fs::read_dir(&dir).with_path(&dir)?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if matches!(name, "manifest.json" | "meta.toml") {
            continue;
        }
        out.push(path);
    }
    out.sort();
    Ok(out)
}

fn should_bundle(pack_type: &str, source_url: Option<&String>) -> bool {
    pack_type == "offline" || (pack_type == "online_local" && source_url.is_none())
}

fn checksum_name(path: &Path) -> Result<String, AppError> {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(str::to_string)
        .ok_or_else(|| AppError::other(format!("Invalid file name: {}", path.display())))
}

pub mod checksum {
    use sha2::{Digest, Sha256};
    use std::fs;
    use std::path::Path;

    use crate::{AppError, WithPath};

    pub fn sha256_bytes(bytes: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        format!("sha256:{:x}", hasher.finalize())
    }

    pub fn sha256_file(path: &Path) -> Result<String, AppError> {
        let bytes = fs::read(path).with_path(path)?;
        Ok(sha256_bytes(&bytes))
    }

    pub fn verify(bytes: &[u8], expected: &str) -> bool {
        sha256_bytes(bytes).eq_ignore_ascii_case(expected)
    }
}

pub mod export {
    use super::*;

    pub fn export_pack(
        app: &AppHandle,
        app_id: String,
        profile_name: String,
        pack_name: String,
        pack_type: String,
        export_path: String,
        include_disabled: bool,
    ) -> Result<String, AppError> {
        let pack_type = match pack_type.as_str() {
            "offline" | "online" | "online_local" => pack_type,
            other => return Err(AppError::other(format!("Unsupported pack type: {}", other))),
        };
        let pack_name_trimmed = pack_name.trim();
        if pack_name_trimmed.is_empty() {
            return Err(AppError::other("Pack name is required"));
        }

        let profile_text = read_profile_text(&app_id, &profile_name)?;
        let parsed_profile = crate::filehandler::parser::parse_profile(&profile_text)
            .map_err(|e| AppError::other(format!("Failed parsing profile: {}", e)))?;
        let game = crate::deployer::read_game_meta(&app_id)?;
        let deployer = parsed_profile.profile.deployer.clone();

        let mut selected = Vec::new();
        for entry in &parsed_profile.modlist {
            let enabled = manifest_enabled(&app_id, &entry.id)?;
            if !include_disabled && !enabled {
                continue;
            }
            if pack_type == "online" && entry.source_url.is_none() {
                emit(
                    app,
                    "warning",
                    &format!("Skipping {}: no source_url", entry.name),
                );
                continue;
            }
            selected.push((entry.clone(), enabled));
        }
        emit(
            app,
            "info",
            &format!("Gathering {} mods for export", selected.len()),
        );

        let mut files_to_checksum = Vec::<(String, PathBuf)>::new();
        for (entry, _) in &selected {
            if should_bundle(&pack_type, entry.source_url.as_ref()) {
                for file in bundled_files(&app_id, &entry.id)? {
                    files_to_checksum.push((entry.id.clone(), file));
                }
            }
        }
        emit(
            app,
            "info",
            &format!("Checksumming {} files", files_to_checksum.len()),
        );

        let mut checked_files: HashMap<String, Vec<(PathBuf, PackFile)>> = HashMap::new();
        let mut total_size_bytes = 0u64;
        for (mod_id, path) in files_to_checksum {
            let size = fs::metadata(&path).with_path(&path)?.len();
            let checksum = checksum::sha256_file(&path)?;
            total_size_bytes += size;
            checked_files.entry(mod_id).or_default().push((
                path.clone(),
                PackFile {
                    name: checksum_name(&path)?,
                    size,
                    checksum,
                },
            ));
        }

        let mut manifest_mods = Vec::new();
        for (entry, enabled) in &selected {
            let files = checked_files
                .get(&entry.id)
                .map(|items| items.iter().map(|(_, f)| f.clone()).collect())
                .unwrap_or_default();
            manifest_mods.push(PackMod {
                id: entry.id.clone(),
                name: entry.name.clone(),
                version: entry.version.clone(),
                source_url: entry.source_url.clone(),
                enabled: *enabled,
                files,
            });
        }

        let manifest = PackManifest {
            pack_format: PACK_FORMAT,
            name: pack_name_trimmed.to_string(),
            pack_type: pack_type.clone(),
            created_at: now_iso(),
            caldera_min_version: CALDERA_MIN_VERSION.to_string(),
            game: PackGame {
                id: app_id.clone(),
                name: game.name,
                deployer,
            },
            profile: profile_name.clone(),
            mods: manifest_mods,
            include_disabled,
            total_mods: selected.len() as u32,
            total_size_bytes,
        };

        emit(app, "info", "Building archive");
        let tar_bytes =
            build_pack(&app_id, &profile_text, &selected, &checked_files, &manifest)?;
        let compressed = zstd::stream::encode_all(Cursor::new(tar_bytes), 3)
            .map_err(|e| AppError::other(format!("Failed compressing pack: {}", e)))?;

        let out_dir = PathBuf::from(export_path);
        fs::create_dir_all(&out_dir).with_path(&out_dir)?;
        let out_path = out_dir.join(format!("{}.caldera", safe_name(pack_name_trimmed)));
        fs::write(&out_path, &compressed).with_path(&out_path)?;
        emit(
            app,
            "success",
            &format!(
                "Pack exported: {} ({:.2}MB)",
                out_path.display(),
                compressed.len() as f64 / 1024.0 / 1024.0
            ),
        );
        Ok(out_path.to_string_lossy().to_string())
    }

    pub fn build_pack(
        app_id: &str,
        profile_text: &str,
        selected: &[(crate::filehandler::parser::ModEntry, bool)],
        checked_files: &HashMap<String, Vec<(PathBuf, PackFile)>>,
        manifest: &PackManifest,
    ) -> Result<Vec<u8>, AppError> {
        let mut builder = Builder::new(Vec::new());
        let manifest_json =
            serde_json::to_vec_pretty(manifest).map_err(AppError::Json)?;
        append_bytes(&mut builder, "pack.json", &manifest_json)?;
        append_bytes(&mut builder, "profile.toml", profile_text.as_bytes())?;

        for (entry, _) in selected {
            append_bytes(
                &mut builder,
                &format!("mods/{}/meta.toml", entry.id),
                read_or_make_meta(app_id, entry)?.as_bytes(),
            )?;
            append_bytes(
                &mut builder,
                &format!("mods/{}/manifest.json", entry.id),
                read_or_make_manifest(app_id, &entry.id)?.as_bytes(),
            )?;
            if let Some(files) = checked_files.get(&entry.id) {
                for (path, file) in files {
                    let bytes = fs::read(path).with_path(path)?;
                    append_bytes(
                        &mut builder,
                        &format!("mods/{}/files/{}", entry.id, file.name),
                        &bytes,
                    )?;
                }
            }
        }

        builder
            .into_inner()
            .map_err(|e| AppError::other(format!("Failed finalizing tar archive: {}", e)))
    }
}

pub mod import {
    use super::*;

    pub fn import_pack(app: &AppHandle, pack_path: String) -> Result<ImportResult, AppError> {
        let p = PathBuf::from(&pack_path);
        let compressed = fs::read(&p).with_path(&p)?;
        let tar_bytes = zstd::stream::decode_all(Cursor::new(compressed))
            .map_err(|e| AppError::other(format!("Failed decompressing pack: {}", e)))?;
        let entries = read_pack_entries(&tar_bytes)?;
        let pack_json = entries
            .get("pack.json")
            .ok_or_else(|| AppError::other("Pack missing pack.json"))?;
        let manifest: PackManifest =
            serde_json::from_slice(pack_json).map_err(AppError::Json)?;
        emit(app, "info", &format!("Reading pack: {}", manifest.name));

        emit(app, "info", "Validating pack");
        if manifest.pack_format != PACK_FORMAT {
            return Err(AppError::other(format!(
                "Unsupported pack format: {}",
                manifest.pack_format
            )));
        }
        if version_gt(&manifest.caldera_min_version, CALDERA_VERSION) {
            return Err(AppError::other(format!(
                "Pack requires CALDERA {}, running {}",
                manifest.caldera_min_version, CALDERA_VERSION
            )));
        }
        if crate::deployer::read_game_meta(&manifest.game.id).is_err() {
            emit(
                app,
                "warning",
                &format!("Game not found in registry: {}", manifest.game.id),
            );
        }

        let failed_mods = verify_bundled_files(app, &manifest, &entries)?;
        let profile_name = imported_profile_name(&manifest.profile);
        write_imported_profile(&manifest.game.id, &profile_name, &entries)?;

        let mut installed = 0u32;
        let mut queued = 0u32;
        let mut failed = failed_mods.len() as u32;
        for mod_entry in &manifest.mods {
            if failed_mods.contains(&mod_entry.id) {
                continue;
            }

            write_mod_metadata(&manifest.game.id, &mod_entry.id, &entries)?;
            if mod_entry.files.is_empty() {
                if let Some(url) = &mod_entry.source_url {
                    emit(app, "info", &format!("queued: {}", url));
                    queued += 1;
                } else {
                    failed += 1;
                }
                continue;
            }

            extract_mod_files(&manifest.game.id, &mod_entry.id, &entries)?;
            installed += 1;
        }

        update_registry_stub(&manifest.game.id, &manifest.mods, &failed_mods, &entries)?;
        emit(
            app,
            "success",
            &format!("Installed {} mods, queued {} downloads", installed, queued),
        );
        emit(app, "success", "Pack import complete");
        Ok(ImportResult {
            mods_installed: installed,
            mods_queued: queued,
            mods_failed: failed,
            profile_name,
        })
    }

    pub fn read_pack(pack_path: &str) -> Result<PackManifest, AppError> {
        let p = PathBuf::from(pack_path);
        let compressed = fs::read(&p).with_path(&p)?;
        let tar_bytes = zstd::stream::decode_all(Cursor::new(compressed))
            .map_err(|e| AppError::other(format!("Failed decompressing pack: {}", e)))?;
        let entries = read_pack_entries(&tar_bytes)?;
        let pack_json = entries
            .get("pack.json")
            .ok_or_else(|| AppError::other("Pack missing pack.json"))?;
        serde_json::from_slice(pack_json).map_err(AppError::Json)
    }

    fn read_pack_entries(tar_bytes: &[u8]) -> Result<HashMap<String, Vec<u8>>, AppError> {
        let mut archive = Archive::new(Cursor::new(tar_bytes));
        let mut out = HashMap::new();
        let entries = archive
            .entries()
            .map_err(|e| AppError::other(format!("Failed reading tar entries: {}", e)))?;
        for entry in entries {
            let mut entry =
                entry.map_err(|e| AppError::other(format!("Invalid tar entry: {}", e)))?;
            if !entry.header().entry_type().is_file() {
                continue;
            }
            let path = entry
                .path()
                .map_err(|e| AppError::other(format!("Invalid tar entry path: {}", e)))?
                .to_string_lossy()
                .replace('\\', "/");
            let mut bytes = Vec::new();
            entry
                .read_to_end(&mut bytes)
                .map_err(|e| AppError::other(format!("Failed reading tar entry {}: {}", path, e)))?;
            out.insert(path, bytes);
        }
        Ok(out)
    }

    fn version_gt(required: &str, running: &str) -> bool {
        let parse = |s: &str| -> Vec<u32> {
            s.split(|c| c == '.' || c == '-')
                .take(3)
                .map(|p| p.parse::<u32>().unwrap_or(0))
                .collect()
        };
        parse(required) > parse(running)
    }

    fn verify_bundled_files(
        app: &AppHandle,
        manifest: &PackManifest,
        entries: &HashMap<String, Vec<u8>>,
    ) -> Result<HashSet<String>, AppError> {
        let mut verified = 0u32;
        let mut failed_mods = HashSet::new();
        for m in &manifest.mods {
            for f in &m.files {
                let path = format!("mods/{}/files/{}", m.id, f.name);
                let Some(bytes) = entries.get(&path) else {
                    emit(app, "error", &format!("Checksum failed: {}", f.name));
                    failed_mods.insert(m.id.clone());
                    continue;
                };
                if checksum::verify(bytes, &f.checksum) {
                    verified += 1;
                } else {
                    emit(app, "error", &format!("Checksum failed: {}", f.name));
                    failed_mods.insert(m.id.clone());
                }
            }
        }
        if failed_mods.is_empty() {
            emit(app, "success", &format!("Verified {} files", verified));
        } else {
            emit(
                app,
                "warning",
                &format!(
                    "Verified {} files with {} failed mod(s)",
                    verified,
                    failed_mods.len()
                ),
            );
        }
        Ok(failed_mods)
    }

    fn imported_profile_name(name: &str) -> String {
        format!("{}_imported", safe_name(name))
    }

    fn write_imported_profile(
        app_id: &str,
        profile_name: &str,
        entries: &HashMap<String, Vec<u8>>,
    ) -> Result<(), AppError> {
        let bytes = entries
            .get("profile.toml")
            .ok_or_else(|| AppError::other("Pack missing profile.toml"))?;
        let path = cache_profile_path(app_id, profile_name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_path(parent)?;
        }
        fs::write(&path, bytes).with_path(&path)
    }

    fn write_mod_metadata(
        app_id: &str,
        mod_id: &str,
        entries: &HashMap<String, Vec<u8>>,
    ) -> Result<(), AppError> {
        let dir = mod_dir(app_id, mod_id).join("files");
        fs::create_dir_all(&dir).with_path(&dir)?;
        for name in ["meta.toml", "manifest.json"] {
            let entry_path = format!("mods/{}/{}", mod_id, name);
            if let Some(bytes) = entries.get(&entry_path) {
                let dest = dir.join(name);
                fs::write(&dest, bytes).with_path(&dest)?;
            }
        }
        Ok(())
    }

    fn extract_mod_files(
        app_id: &str,
        mod_id: &str,
        entries: &HashMap<String, Vec<u8>>,
    ) -> Result<(), AppError> {
        let dir = mod_dir(app_id, mod_id);
        fs::create_dir_all(&dir).with_path(&dir)?;
        let prefix = format!("mods/{}/files/", mod_id);
        for (path, bytes) in entries.iter().filter(|(path, _)| path.starts_with(&prefix)) {
            let filename = path.trim_start_matches(&prefix);
            if filename.contains('/') || filename.contains('\\') || filename.is_empty() {
                continue;
            }
            let dest = dir.join(filename);
            fs::write(&dest, bytes).with_path(&dest)?;
        }
        Ok(())
    }

    fn update_registry_stub(
        app_id: &str,
        mods: &[PackMod],
        failed_mods: &HashSet<String>,
        entries: &HashMap<String, Vec<u8>>,
    ) -> Result<(), AppError> {
        let path = crate::filehandler::runtime_reader::registry_path();
        let mut registry = if path.exists() {
            let raw = fs::read_to_string(&path).with_path(&path)?;
            serde_json::from_str::<serde_json::Value>(&raw).map_err(AppError::Json)?
        } else {
            serde_json::json!({ "version": 1, "games": {} })
        };

        let games = registry
            .get_mut("games")
            .and_then(|v| v.as_object_mut())
            .ok_or_else(|| AppError::other("Invalid registry: missing games object"))?;
        let game = games
            .entry(app_id.to_string())
            .or_insert_with(|| serde_json::json!({ "mods": {} }));
        let game_mods = game
            .get_mut("mods")
            .and_then(|v| v.as_object_mut())
            .ok_or_else(|| AppError::other("Invalid registry: missing game mods object"))?;

        for m in mods {
            if failed_mods.contains(&m.id) || m.files.is_empty() {
                continue;
            }
            let manifest_path = format!("mods/{}/manifest.json", m.id);
            let manifest = entries
                .get(&manifest_path)
                .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(bytes).ok())
                .unwrap_or_else(|| serde_json::json!({ "deployed": false, "files": [] }));
            game_mods.insert(m.id.clone(), manifest);
        }

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_path(parent)?;
        }
        let body = serde_json::to_string_pretty(&registry).map_err(AppError::Json)?;
        fs::write(&path, body).with_path(&path)
    }
}

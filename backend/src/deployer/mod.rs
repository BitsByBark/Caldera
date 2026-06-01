use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

pub mod unreal;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployerConfig {
    pub id: String,
    pub name: String,
    pub description: String,
    pub content_path_hint: String,
    pub mod_subfolder: String,
    pub create_mod_folder: bool,
    pub file_patterns: Vec<String>,
    pub group_by_basename: bool,
    pub load_order: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeployerOption {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModManifest {
    pub deployed: bool,
    pub deployer: String,
    pub target_folder: String,
    pub files: Vec<DeployedFile>,
    pub deployed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployedFile {
    pub name: String,
    pub target: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RegistryFile {
    version: u32,
    games: HashMap<String, RegistryGame>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct RegistryGame {
    mods: HashMap<String, RegistryMod>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RegistryMod {
    deployer: String,
    target_folder: String,
    files: Vec<RegistryFileEntry>,
    deployed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RegistryFileEntry {
    name: String,
    target: String,
    enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
struct LogEvent {
    message: String,
    level: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GameMeta {
    pub app_id: String,
    pub name: String,
    pub install_path: String,
}

pub trait DeployLogger {
    fn info(&self, message: &str);
    fn success(&self, message: &str);
    fn warning(&self, message: &str);
    fn error(&self, message: &str);
}

struct TauriLogger {
    app: AppHandle,
}

impl TauriLogger {
    fn emit_level(&self, level: &str, message: &str) {
        let payload = LogEvent {
            message: message.to_string(),
            level: level.to_string(),
        };
        let _ = self.app.emit("caldera://session-log", payload);
    }
}

impl DeployLogger for TauriLogger {
    fn info(&self, message: &str) {
        self.emit_level("info", message);
    }
    fn success(&self, message: &str) {
        self.emit_level("success", message);
    }
    fn warning(&self, message: &str) {
        self.emit_level("warning", message);
    }
    fn error(&self, message: &str) {
        self.emit_level("error", message);
    }
}

fn base_config_dir() -> PathBuf {
    crate::runtime::base_config_dir()
}

fn cache_root(app_id: &str) -> PathBuf {
    base_config_dir().join("cache").join(app_id)
}

fn mod_storage_dir(app_id: &str, mod_id: &str) -> PathBuf {
    cache_root(app_id).join("mods").join(mod_id)
}

fn manifest_path(app_id: &str, mod_id: &str) -> PathBuf {
    mod_storage_dir(app_id, mod_id).join("manifest.json")
}

fn registry_path() -> PathBuf {
    base_config_dir().join("registry.caldera")
}

fn game_cache_config_path(app_id: &str) -> PathBuf {
    cache_root(app_id).join("config.toml")
}

fn game_meta_path(app_id: &str) -> PathBuf {
    cache_root(app_id).join("meta.json")
}

fn rfc3339_now_utc() -> Option<String> {
    OffsetDateTime::now_utc().format(&Rfc3339).ok()
}

fn load_registry() -> Result<RegistryFile, String> {
    let p = registry_path();
    if !p.exists() {
        return Ok(RegistryFile {
            version: 1,
            games: HashMap::new(),
        });
    }

    let raw = fs::read_to_string(&p)
        .map_err(|e| format!("Failed reading registry {}: {}", p.display(), e))?;
    let parsed: RegistryFile =
        serde_json::from_str(&raw).map_err(|e| format!("Invalid registry JSON: {}", e))?;
    Ok(parsed)
}

fn save_registry(registry: &RegistryFile) -> Result<(), String> {
    let p = registry_path();
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed creating registry dir {}: {}", parent.display(), e))?;
    }
    let body = serde_json::to_string_pretty(registry)
        .map_err(|e| format!("Failed serializing registry: {}", e))?;
    fs::write(&p, body).map_err(|e| format!("Failed writing registry {}: {}", p.display(), e))
}

fn load_manifest(app_id: &str, mod_id: &str) -> Result<Option<ModManifest>, String> {
    let p = manifest_path(app_id, mod_id);
    if !p.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&p)
        .map_err(|e| format!("Failed reading manifest {}: {}", p.display(), e))?;
    let parsed = serde_json::from_str::<ModManifest>(&raw)
        .map_err(|e| format!("Invalid manifest JSON {}: {}", p.display(), e))?;
    Ok(Some(parsed))
}

fn save_manifest(app_id: &str, mod_id: &str, manifest: &ModManifest) -> Result<(), String> {
    let p = manifest_path(app_id, mod_id);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed creating mod storage dir {}: {}", parent.display(), e))?;
    }
    let body = serde_json::to_string_pretty(manifest)
        .map_err(|e| format!("Failed serializing manifest: {}", e))?;
    fs::write(&p, body).map_err(|e| format!("Failed writing manifest {}: {}", p.display(), e))
}

pub fn read_game_meta(app_id: &str) -> Result<GameMeta, String> {
    let p = game_meta_path(app_id);
    let raw = fs::read_to_string(&p)
        .map_err(|e| format!("Failed reading game meta {}: {}", p.display(), e))?;
    serde_json::from_str::<GameMeta>(&raw).map_err(|e| format!("Invalid game meta JSON: {}", e))
}

pub fn read_game_cache_config(app_id: &str) -> Result<toml::Value, String> {
    let p = game_cache_config_path(app_id);
    if !p.exists() {
        return Ok(toml::Value::Table(toml::map::Map::new()));
    }
    let raw = fs::read_to_string(&p)
        .map_err(|e| format!("Failed reading cache config {}: {}", p.display(), e))?;
    if raw.trim().is_empty() {
        return Ok(toml::Value::Table(toml::map::Map::new()));
    }
    toml::from_str::<toml::Value>(&raw)
        .map_err(|e| format!("Invalid cache config TOML {}: {}", p.display(), e))
}

pub fn write_game_cache_config(app_id: &str, cfg: &toml::Value) -> Result<(), String> {
    let p = game_cache_config_path(app_id);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed creating cache config dir {}: {}", parent.display(), e))?;
    }
    let body = toml::to_string(cfg).map_err(|e| format!("Failed serializing TOML: {}", e))?;
    fs::write(&p, body).map_err(|e| format!("Failed writing cache config {}: {}", p.display(), e))
}

pub fn get_deployer_override_path(app_id: &str) -> Result<Option<PathBuf>, String> {
    let cfg = read_game_cache_config(app_id)?;
    let val = cfg
        .get("deployer_mod_path")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string());

    Ok(val
        .filter(|s| !s.is_empty())
        .map(PathBuf::from))
}

fn get_selected_deployer(app_id: &str) -> Result<Option<String>, String> {
    let cfg = read_game_cache_config(app_id)?;
    Ok(cfg
        .get("deployer")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty()))
}

fn load_deployer_config(app: &AppHandle, deployer_id: &str) -> Result<DeployerConfig, String> {
    let dev_path = PathBuf::from("../defaults")
        .join("deployers")
        .join(format!("{}.toml", deployer_id));

    let bundle_path = app
        .path()
        .resource_dir()
        .map_err(|e| format!("Failed reading resource dir: {}", e))?
        .join("defaults")
        .join("deployers")
        .join(format!("{}.toml", deployer_id));

    let source = if dev_path.exists() {
        dev_path
    } else if bundle_path.exists() {
        bundle_path
    } else {
        return Err(format!(
            "Deployer config not found for '{}'. Looked in '{}' and '{}'",
            deployer_id,
            PathBuf::from("../defaults/deployers").display(),
            app.path()
                .resource_dir()
                .map(|p| p.join("defaults/deployers").display().to_string())
                .unwrap_or_else(|_| "<resource_dir unavailable>".to_string())
        ));
    };

    let raw = fs::read_to_string(&source)
        .map_err(|e| format!("Failed reading deployer config {}: {}", source.display(), e))?;
    toml::from_str::<DeployerConfig>(&raw)
        .map_err(|e| format!("Invalid deployer TOML {}: {}", source.display(), e))
}

fn deployer_dirs(app: &AppHandle) -> Result<Vec<PathBuf>, String> {
    let dev = PathBuf::from("../defaults").join("deployers");
    let bundle = app
        .path()
        .resource_dir()
        .map_err(|e| format!("Failed reading resource dir: {}", e))?
        .join("defaults")
        .join("deployers");
    Ok(vec![dev, bundle])
}

pub fn get_available_deployers(app: &AppHandle) -> Result<Vec<DeployerOption>, String> {
    let mut items: HashMap<String, DeployerOption> = HashMap::new();

    for dir in deployer_dirs(app)? {
        if !dir.is_dir() {
            continue;
        }

        let entries = fs::read_dir(&dir)
            .map_err(|e| format!("Failed reading deployers dir {}: {}", dir.display(), e))?;
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) != Some("toml") {
                continue;
            }

            let raw = fs::read_to_string(&p)
                .map_err(|e| format!("Failed reading deployer file {}: {}", p.display(), e))?;
            let cfg = toml::from_str::<DeployerConfig>(&raw)
                .map_err(|e| format!("Invalid deployer TOML {}: {}", p.display(), e))?;

            items.insert(
                cfg.id.clone(),
                DeployerOption {
                    id: cfg.id,
                    name: cfg.name,
                },
            );
        }
    }

    let mut out: Vec<DeployerOption> = items.into_values().collect();
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(out)
}

pub fn get_configured_deployer(app_id: &str) -> Result<Option<String>, String> {
    get_selected_deployer(app_id)
}

pub fn set_game_deployer(app: &AppHandle, app_id: &str, deployer_id: &str) -> Result<(), String> {
    if deployer_id != "NONE" {
        let valid = get_available_deployers(app)?
            .into_iter()
            .any(|d| d.id == deployer_id);
        if !valid {
            return Err(format!("Unknown deployer id: {}", deployer_id));
        }
    }

    let mut cfg = read_game_cache_config(app_id)?;
    let table = cfg
        .as_table_mut()
        .ok_or_else(|| "Cache config root must be a TOML table".to_string())?;
    table.insert(
        "deployer".to_string(),
        toml::Value::String(deployer_id.to_string()),
    );
    write_game_cache_config(app_id, &cfg)
}

fn ensure_game_not_running(logger: &impl DeployLogger) -> Result<(), String> {
    logger.warning("Process detection is currently stubbed in this pass");
    Ok(())
}

fn matches_patterns(name: &str, patterns: &[String]) -> bool {
    let lower = name.to_ascii_lowercase();
    patterns.iter().any(|p| {
        let p = p.to_ascii_lowercase();
        if let Some(ext) = p.strip_prefix("*.") {
            lower.ends_with(&format!(".{}", ext))
        } else {
            lower == p
        }
    })
}

fn gather_mod_files(dir: &Path, cfg: &DeployerConfig) -> Result<Vec<PathBuf>, String> {
    if !dir.exists() {
        return Err(format!("Mod storage folder not found: {}", dir.display()));
    }

    let mut out = Vec::new();
    let entries = fs::read_dir(dir)
        .map_err(|e| format!("Failed reading mod storage folder {}: {}", dir.display(), e))?;
    for entry in entries.flatten() {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        let Some(name) = p.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if matches_patterns(name, &cfg.file_patterns) {
            out.push(p);
        }
    }

    out.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
    Ok(out)
}

fn listing_candidate_names(listing: &crate::config::ModListing) -> Vec<String> {
    let mut out = Vec::new();
    if !listing.files.is_empty() {
        out.extend(listing.files.clone());
    }
    if let Some(p) = &listing.source_path {
        if let Some(name) = PathBuf::from(p)
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
        {
            out.push(name);
        }
    }
    if out.is_empty() {
        out.push(listing.name.clone());
    }
    out
}

pub fn listing_matches_deployer(listing: &crate::config::ModListing, cfg: &DeployerConfig) -> bool {
    let candidates = listing_candidate_names(listing);
    candidates
        .iter()
        .any(|name| matches_patterns(name, &cfg.file_patterns))
}

pub fn annotate_modlist_with_deployer(
    app: &AppHandle,
    app_id: &str,
    mut listings: Vec<crate::config::ModListing>,
) -> Result<Vec<crate::config::ModListing>, String> {
    let selected = get_selected_deployer(app_id)?;
    let Some(deployer_id) = selected else {
        for l in &mut listings {
            l.deployable = false;
            l.deployer_reason = Some("No deployer configured".to_string());
        }
        return Ok(listings);
    };
    if deployer_id == "NONE" {
        for l in &mut listings {
            l.deployable = false;
            l.deployer_reason = Some("No deployer configured".to_string());
        }
        return Ok(listings);
    }

    let cfg = load_deployer_config(app, &deployer_id)?;
    for l in &mut listings {
        let ok = listing_matches_deployer(l, &cfg);
        l.deployable = ok;
        l.deployer_reason = if ok {
            None
        } else {
            Some("Selected deployer does not accept this listing file type".to_string())
        };
    }
    Ok(listings)
}

fn update_registry_from_manifest(
    app_id: &str,
    mod_id: &str,
    manifest: &ModManifest,
) -> Result<(), String> {
    let mut reg = load_registry()?;

    let game = reg
        .games
        .entry(app_id.to_string())
        .or_insert_with(RegistryGame::default);

    let entry = RegistryMod {
        deployer: manifest.deployer.clone(),
        target_folder: manifest.target_folder.clone(),
        files: manifest
            .files
            .iter()
            .map(|f| RegistryFileEntry {
                name: f.name.clone(),
                target: f.target.clone(),
                enabled: f.enabled,
            })
            .collect(),
        deployed_at: manifest.deployed_at.clone(),
    };

    game.mods.insert(mod_id.to_string(), entry);
    save_registry(&reg)
}

fn remove_registry_mod(app_id: &str, mod_id: &str) -> Result<(), String> {
    let mut reg = load_registry()?;
    if let Some(game) = reg.games.get_mut(app_id) {
        game.mods.remove(mod_id);
    }
    save_registry(&reg)
}

pub fn recover_registry_state(app_id: &str, logger: &impl DeployLogger) -> Result<(), String> {
    let mut reg = load_registry()?;
    let Some(game) = reg.games.get_mut(app_id) else {
        return Ok(());
    };

    for (mod_id, mod_entry) in &mut game.mods {
        for file in &mut mod_entry.files {
            let target = PathBuf::from(&file.target);
            let disabled = PathBuf::from(format!("{}.disabled", file.target));
            if target.exists() {
                file.enabled = true;
            } else if disabled.exists() {
                file.enabled = false;
            } else {
                logger.warning(&format!("Missing deployed files for {}", mod_id));
            }
        }

        let manifest = ModManifest {
            deployed: true,
            deployer: mod_entry.deployer.clone(),
            target_folder: mod_entry.target_folder.clone(),
            files: mod_entry
                .files
                .iter()
                .map(|f| DeployedFile {
                    name: f.name.clone(),
                    target: f.target.clone(),
                    enabled: f.enabled,
                })
                .collect(),
            deployed_at: mod_entry.deployed_at.clone(),
        };
        let _ = save_manifest(app_id, mod_id, &manifest);
    }

    save_registry(&reg)
}

pub fn resolve_deployer_path(
    app: &AppHandle,
    app_id: String,
    deployer_id: String,
) -> Result<String, String> {
    let logger = TauriLogger { app: app.clone() };
    recover_registry_state(&app_id, &logger)?;

    let cfg = load_deployer_config(app, &deployer_id)?;
    match deployer_id.as_str() {
        "unreal_engine" => unreal::resolve_unreal_mod_path(&app_id, &cfg, &logger)
            .map(|p| p.to_string_lossy().to_string()),
        _ => Err(format!("Unsupported deployer: {}", deployer_id)),
    }
}

pub fn deploy_mod(app: &AppHandle, app_id: String, mod_id: String) -> Result<ModManifest, String> {
    let logger = TauriLogger { app: app.clone() };
    recover_registry_state(&app_id, &logger)?;

    let selected = get_selected_deployer(&app_id)?;
    let Some(deployer_id) = selected else {
        logger.error("No deployer configured");
        return Err("No deployer configured".to_string());
    };

    if deployer_id == "NONE" {
        logger.error("No deployer configured");
        return Err("No deployer configured".to_string());
    }

    ensure_game_not_running(&logger)?;

    let cfg = load_deployer_config(app, &deployer_id)?;
    let target_dir = match deployer_id.as_str() {
        "unreal_engine" => unreal::resolve_unreal_mod_path(&app_id, &cfg, &logger)?,
        _ => return Err(format!("Unsupported deployer: {}", deployer_id)),
    };

    if cfg.create_mod_folder {
        fs::create_dir_all(&target_dir)
            .map_err(|e| format!("Failed creating target mod dir {}: {}", target_dir.display(), e))?;
    }

    logger.info(&format!("Deploying to {}", target_dir.display()));

    let source_dir = mod_storage_dir(&app_id, &mod_id);
    let files = gather_mod_files(&source_dir, &cfg)?;
    logger.info(&format!("Found {} pak file(s)", files.len()));

    let mut deployed_files: Vec<DeployedFile> = Vec::new();
    let mut copied_count = 0usize;

    for src in files {
        let Some(name) = src.file_name().and_then(|n| n.to_str()) else {
            continue;
        };

        let target = target_dir.join(name);
        if target.exists() {
            logger.warning(&format!("Conflict: {} already exists", name));
            continue;
        }

        fs::copy(&src, &target)
            .map_err(|e| format!("Failed copying {} to {}: {}", src.display(), target.display(), e))?;

        copied_count += 1;
        logger.success(&format!("Copied {}", name));
        deployed_files.push(DeployedFile {
            name: name.to_string(),
            target: target.to_string_lossy().to_string(),
            enabled: true,
        });
    }

    let manifest = ModManifest {
        deployed: true,
        deployer: deployer_id,
        target_folder: target_dir.to_string_lossy().to_string(),
        files: deployed_files,
        deployed_at: rfc3339_now_utc(),
    };

    save_manifest(&app_id, &mod_id, &manifest)?;
    update_registry_from_manifest(&app_id, &mod_id, &manifest)?;
    logger.success(&format!("Deployment complete — {} files copied", copied_count));

    Ok(manifest)
}

pub fn undeploy_mod(app: &AppHandle, app_id: String, mod_id: String) -> Result<(), String> {
    let logger = TauriLogger { app: app.clone() };
    recover_registry_state(&app_id, &logger)?;

    ensure_game_not_running(&logger)?;

    let existing = load_manifest(&app_id, &mod_id)?;
    let Some(mut manifest) = existing else {
        remove_registry_mod(&app_id, &mod_id)?;
        logger.success("Mod removed");
        return Ok(());
    };

    if manifest.deployed {
        for f in &manifest.files {
            let target = PathBuf::from(&f.target);
            let disabled = PathBuf::from(format!("{}.disabled", f.target));

            if target.exists() {
                fs::remove_file(&target)
                    .map_err(|e| format!("Failed removing {}: {}", target.display(), e))?;
                logger.info(&format!("Removed {}", f.name));
            } else if disabled.exists() {
                fs::remove_file(&disabled)
                    .map_err(|e| format!("Failed removing {}: {}", disabled.display(), e))?;
                logger.info(&format!("Removed {}", f.name));
            }
        }
    }

    manifest.deployed = false;
    manifest.target_folder = String::new();
    manifest.files.clear();
    manifest.deployed_at = None;

    save_manifest(&app_id, &mod_id, &manifest)?;
    remove_registry_mod(&app_id, &mod_id)?;
    logger.success("Mod removed");

    Ok(())
}

pub fn toggle_mod(
    app: &AppHandle,
    app_id: String,
    mod_id: String,
    enabled: bool,
) -> Result<(), String> {
    let logger = TauriLogger { app: app.clone() };
    recover_registry_state(&app_id, &logger)?;

    ensure_game_not_running(&logger)?;

    let existing = load_manifest(&app_id, &mod_id)?;
    let Some(mut manifest) = existing else {
        return Err("Mod manifest not found".to_string());
    };

    if !manifest.deployed {
        return Err("Mod is not deployed".to_string());
    }

    for f in &mut manifest.files {
        let active = PathBuf::from(&f.target);
        let disabled_path = PathBuf::from(format!("{}.disabled", f.target));

        if enabled {
            if disabled_path.exists() {
                fs::rename(&disabled_path, &active).map_err(|e| {
                    format!(
                        "Failed enabling {} ({} -> {}): {}",
                        f.name,
                        disabled_path.display(),
                        active.display(),
                        e
                    )
                })?;
            }
            f.enabled = true;
        } else {
            if active.exists() {
                fs::rename(&active, &disabled_path).map_err(|e| {
                    format!(
                        "Failed disabling {} ({} -> {}): {}",
                        f.name,
                        active.display(),
                        disabled_path.display(),
                        e
                    )
                })?;
            }
            f.enabled = false;
        }
    }

    save_manifest(&app_id, &mod_id, &manifest)?;
    update_registry_from_manifest(&app_id, &mod_id, &manifest)?;

    if enabled {
        logger.info(&format!("{} enabled", mod_id));
    } else {
        logger.info(&format!("{} disabled", mod_id));
    }

    Ok(())
}

pub fn set_deployer_override_path(app_id: &str, path: &Path) -> Result<(), String> {
    let mut cfg = read_game_cache_config(app_id)?;
    let obj = cfg
        .as_table_mut()
        .ok_or_else(|| "Cache config root must be a TOML table".to_string())?;
    obj.insert(
        "deployer_mod_path".to_string(),
        toml::Value::String(path.to_string_lossy().to_string()),
    );
    write_game_cache_config(app_id, &cfg)
}

pub fn patch_game_cache_config(app_id: &str, patch: HashMap<String, Value>) -> Result<(), String> {
    let mut cfg = read_game_cache_config(app_id)?;
    let obj = cfg
        .as_table_mut()
        .ok_or_else(|| "Cache config root must be a TOML table".to_string())?;

    for (k, v) in patch {
        let toml_value = match v {
            Value::String(s) => toml::Value::String(s),
            Value::Bool(b) => toml::Value::Boolean(b),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    toml::Value::Integer(i)
                } else if let Some(f) = n.as_f64() {
                    toml::Value::Float(f)
                } else {
                    continue;
                }
            }
            _ => continue,
        };
        obj.insert(k, toml_value);
    }

    write_game_cache_config(app_id, &cfg)
}

pub fn deploy_listing(
    app: &AppHandle,
    app_id: String,
    listing_id: String,
) -> Result<ModManifest, String> {
    let logger = TauriLogger { app: app.clone() };
    recover_registry_state(&app_id, &logger)?;

    let selected = get_selected_deployer(&app_id)?;
    let Some(deployer_id) = selected else {
        logger.error("No deployer configured");
        return Err("No deployer configured".to_string());
    };
    if deployer_id == "NONE" {
        logger.error("No deployer configured");
        return Err("No deployer configured".to_string());
    }

    let cfg = load_deployer_config(app, &deployer_id)?;
    let raw = crate::config::get_raw_modlist_listings(app_id.clone())?;
    let listings = annotate_modlist_with_deployer(app, &app_id, raw)?;
    let listing = listings
        .into_iter()
        .find(|l| l.mod_id == listing_id)
        .ok_or_else(|| format!("Listing not found: {}", listing_id))?;

    if !listing.deployable {
        let reason = listing
            .deployer_reason
            .clone()
            .unwrap_or_else(|| "Listing is not accepted by selected deployer".to_string());
        logger.warning(&reason);
        return Err(reason);
    }

    ensure_game_not_running(&logger)?;

    let mod_id = listing.mod_id.clone();
    let mod_dir = mod_storage_dir(&app_id, &mod_id);
    fs::create_dir_all(&mod_dir)
        .map_err(|e| format!("Failed creating mod staging dir {}: {}", mod_dir.display(), e))?;

    let mut copied = 0usize;
    let mut seen = HashSet::new();
    let source_parent = listing
        .source_path
        .as_ref()
        .and_then(|p| PathBuf::from(p).parent().map(|p| p.to_path_buf()));

    for file_name in listing_candidate_names(&listing) {
        if !matches_patterns(&file_name, &cfg.file_patterns) {
            continue;
        }
        if !seen.insert(file_name.clone()) {
            continue;
        }

        let src = if let Some(sp) = &listing.source_path {
            let spb = PathBuf::from(sp);
            if spb
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n == file_name)
                .unwrap_or(false)
            {
                spb
            } else if let Some(parent) = &source_parent {
                parent.join(&file_name)
            } else {
                continue;
            }
        } else if let Some(parent) = &source_parent {
            parent.join(&file_name)
        } else {
            continue;
        };

        if !src.exists() || !src.is_file() {
            continue;
        }
        let target = mod_dir.join(&file_name);
        fs::copy(&src, &target).map_err(|e| {
            format!(
                "Failed staging {} -> {}: {}",
                src.display(),
                target.display(),
                e
            )
        })?;
        copied += 1;
    }

    if copied == 0 {
        return Err("No deployable files were staged from listing".to_string());
    }

    logger.info(&format!("Staged {} file(s) for {}", copied, mod_id));
    let manifest = deploy_mod(app, app_id.clone(), mod_id.clone())?;
    crate::profile_runtime::upsert_profile_from_deploy(&app_id, &deployer_id, &listing, &manifest)?;
    logger.success(&format!("Profile updated for {}", mod_id));
    Ok(manifest)
}

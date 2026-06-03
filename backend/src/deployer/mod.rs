use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::{AppError, WithPath};

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
    #[serde(default, alias = "enabled")]
    pub linked: bool,
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
    #[serde(default, alias = "enabled")]
    linked: bool,
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

fn mod_files_dir(app_id: &str, mod_id: &str) -> PathBuf {
    crate::filehandler::runtime_reader::mod_files_dir(app_id, mod_id)
}

fn manifest_path(app_id: &str, mod_id: &str) -> PathBuf {
    crate::filehandler::runtime_reader::mod_manifest_path(app_id, mod_id)
}

fn registry_path() -> PathBuf {
    crate::filehandler::runtime_reader::registry_path()
}

fn game_cache_config_path(app_id: &str) -> PathBuf {
    crate::filehandler::runtime_reader::game_config_path(app_id)
}

fn game_meta_path(app_id: &str) -> PathBuf {
    crate::filehandler::runtime_reader::game_meta_path(app_id)
}

fn rfc3339_now_utc() -> Option<String> {
    OffsetDateTime::now_utc().format(&Rfc3339).ok()
}

fn load_registry() -> Result<RegistryFile, AppError> {
    let p = registry_path();
    if !p.exists() {
        return Ok(RegistryFile {
            version: 1,
            games: HashMap::new(),
        });
    }
    let raw = fs::read_to_string(&p).with_path(&p)?;
    serde_json::from_str(&raw).map_err(AppError::Json)
}

fn save_registry(registry: &RegistryFile) -> Result<(), AppError> {
    let p = registry_path();
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).with_path(parent)?;
    }
    let body = serde_json::to_string_pretty(registry).map_err(AppError::Json)?;
    fs::write(&p, body).with_path(&p)
}

fn load_manifest(app_id: &str, mod_id: &str) -> Result<Option<ModManifest>, AppError> {
    let p = manifest_path(app_id, mod_id);
    if !p.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&p).with_path(&p)?;
    let parsed = serde_json::from_str::<ModManifest>(&raw).map_err(AppError::Json)?;
    Ok(Some(parsed))
}

fn save_manifest(app_id: &str, mod_id: &str, manifest: &ModManifest) -> Result<(), AppError> {
    let p = manifest_path(app_id, mod_id);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).with_path(parent)?;
    }
    let body = serde_json::to_string_pretty(manifest).map_err(AppError::Json)?;
    fs::write(&p, body).with_path(&p)
}

#[cfg(unix)]
fn create_file_link(
    source: &Path,
    target: &Path,
    _logger: &impl DeployLogger,
) -> Result<(), AppError> {
    std::os::unix::fs::symlink(source, target).with_path(target)
}

#[cfg(windows)]
fn create_file_link(
    source: &Path,
    target: &Path,
    logger: &impl DeployLogger,
) -> Result<(), AppError> {
    match std::os::windows::fs::symlink_file(source, target) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            // TODO: request elevation via Tauri, then retry the symlink.
            let msg = "Symlinks require admin rights or Developer Mode — enable Developer Mode in Windows Settings > For Developers";
            logger.error(msg);
            Err(AppError::other(msg))
        }
        Err(e) => Err(AppError::Io {
            path: target.to_path_buf(),
            source: e,
        }),
    }
}

fn remove_link_if_present(target: &Path) -> Result<bool, AppError> {
    if target.symlink_metadata().is_ok() {
        fs::remove_file(target).with_path(target)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

fn source_file_path(app_id: &str, mod_id: &str, name: &str) -> PathBuf {
    mod_files_dir(app_id, mod_id).join(name)
}

pub fn read_game_meta(app_id: &str) -> Result<GameMeta, AppError> {
    let p = game_meta_path(app_id);
    let raw = fs::read_to_string(&p).with_path(&p)?;
    serde_json::from_str::<GameMeta>(&raw).map_err(AppError::Json)
}

pub fn read_game_cache_config(app_id: &str) -> Result<toml::Value, AppError> {
    let p = game_cache_config_path(app_id);
    if !p.exists() {
        return Ok(toml::Value::Table(toml::map::Map::new()));
    }
    let raw = fs::read_to_string(&p).with_path(&p)?;
    if raw.trim().is_empty() {
        return Ok(toml::Value::Table(toml::map::Map::new()));
    }
    toml::from_str::<toml::Value>(&raw).map_err(AppError::TomlParse)
}

pub fn write_game_cache_config(app_id: &str, cfg: &toml::Value) -> Result<(), AppError> {
    let p = game_cache_config_path(app_id);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).with_path(parent)?;
    }
    let body = toml::to_string(cfg).map_err(AppError::TomlSerialize)?;
    fs::write(&p, body).with_path(&p)
}

pub fn get_deployer_override_path(app_id: &str) -> Result<Option<PathBuf>, AppError> {
    let cfg = read_game_cache_config(app_id)?;
    let val = cfg
        .get("deployer_mod_path")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string());

    Ok(val.filter(|s| !s.is_empty()).map(PathBuf::from))
}

fn get_selected_deployer(app_id: &str) -> Result<Option<String>, AppError> {
    let cfg = read_game_cache_config(app_id)?;
    Ok(cfg
        .get("deployer")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty()))
}

fn load_deployer_config(app: &AppHandle, deployer_id: &str) -> Result<DeployerConfig, AppError> {
    let dev_path = PathBuf::from("../defaults")
        .join("deployers")
        .join(format!("{}.toml", deployer_id));

    let bundle_path = app
        .path()
        .resource_dir()
        .map_err(|e| AppError::other(format!("Failed reading resource dir: {}", e)))?
        .join("defaults")
        .join("deployers")
        .join(format!("{}.toml", deployer_id));

    let source = if dev_path.exists() {
        dev_path
    } else if bundle_path.exists() {
        bundle_path
    } else {
        return Err(AppError::other(format!(
            "Deployer config not found for '{}'. Looked in '{}' and '{}'",
            deployer_id,
            PathBuf::from("../defaults/deployers").display(),
            app.path()
                .resource_dir()
                .map(|p| p.join("defaults/deployers").display().to_string())
                .unwrap_or_else(|_| "<resource_dir unavailable>".to_string())
        )));
    };

    let raw = fs::read_to_string(&source).with_path(&source)?;
    toml::from_str::<DeployerConfig>(&raw).map_err(AppError::TomlParse)
}

fn deployer_dirs(app: &AppHandle) -> Result<Vec<PathBuf>, AppError> {
    let dev = PathBuf::from("../defaults").join("deployers");
    let bundle = app
        .path()
        .resource_dir()
        .map_err(|e| AppError::other(format!("Failed reading resource dir: {}", e)))?
        .join("defaults")
        .join("deployers");
    Ok(vec![dev, bundle])
}

pub fn get_available_deployers(app: &AppHandle) -> Result<Vec<DeployerOption>, AppError> {
    let mut items: HashMap<String, DeployerOption> = HashMap::new();

    for dir in deployer_dirs(app)? {
        if !dir.is_dir() {
            continue;
        }

        let entries = fs::read_dir(&dir).with_path(&dir)?;
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) != Some("toml") {
                continue;
            }

            let raw = fs::read_to_string(&p).with_path(&p)?;
            let cfg = toml::from_str::<DeployerConfig>(&raw).map_err(AppError::TomlParse)?;

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

pub fn get_configured_deployer(app_id: &str) -> Result<Option<String>, AppError> {
    get_selected_deployer(app_id)
}

pub fn set_game_deployer(
    app: &AppHandle,
    app_id: &str,
    deployer_id: &str,
) -> Result<(), AppError> {
    if deployer_id != "NONE" {
        let valid = get_available_deployers(app)?
            .into_iter()
            .any(|d| d.id == deployer_id);
        if !valid {
            return Err(AppError::other(format!("Unknown deployer id: {}", deployer_id)));
        }
    }

    let mut cfg = read_game_cache_config(app_id)?;
    let table = cfg
        .as_table_mut()
        .ok_or_else(|| AppError::other("Cache config root must be a TOML table"))?;
    table.insert(
        "deployer".to_string(),
        toml::Value::String(deployer_id.to_string()),
    );
    write_game_cache_config(app_id, &cfg)
}

fn ensure_game_not_running(logger: &impl DeployLogger) -> Result<(), AppError> {
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

fn gather_mod_files(dir: &Path, cfg: &DeployerConfig) -> Result<Vec<PathBuf>, AppError> {
    if !dir.exists() {
        return Err(AppError::other(format!(
            "Mod storage folder not found: {}",
            dir.display()
        )));
    }

    let mut out = Vec::new();
    let entries = fs::read_dir(dir).with_path(dir)?;
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

pub fn listing_matches_deployer(
    listing: &crate::config::ModListing,
    cfg: &DeployerConfig,
) -> bool {
    let candidates = listing_candidate_names(listing);
    candidates
        .iter()
        .any(|name| matches_patterns(name, &cfg.file_patterns))
}

pub fn annotate_modlist_with_deployer(
    app: &AppHandle,
    app_id: &str,
    mut listings: Vec<crate::config::ModListing>,
) -> Result<Vec<crate::config::ModListing>, AppError> {
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
) -> Result<(), AppError> {
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
                linked: f.linked,
            })
            .collect(),
        deployed_at: manifest.deployed_at.clone(),
    };

    game.mods.insert(mod_id.to_string(), entry);
    save_registry(&reg)
}

fn remove_registry_mod(app_id: &str, mod_id: &str) -> Result<(), AppError> {
    let mut reg = load_registry()?;
    if let Some(game) = reg.games.get_mut(app_id) {
        game.mods.remove(mod_id);
    }
    save_registry(&reg)
}

pub fn recover_registry_state(
    app_id: &str,
    logger: &impl DeployLogger,
) -> Result<(), AppError> {
    let mut reg = load_registry()?;
    let Some(game) = reg.games.get_mut(app_id) else {
        return Ok(());
    };

    for (mod_id, mod_entry) in &mut game.mods {
        for file in &mut mod_entry.files {
            let target = PathBuf::from(&file.target);
            let source = source_file_path(app_id, mod_id, &file.name);
            if target.symlink_metadata().is_ok() {
                file.linked = true;
            } else if source.exists() {
                file.linked = false;
                logger.warning(&format!("Mod {} is undeployed", file.name));
            } else {
                file.linked = false;
                logger.error(&format!("Missing source file for {}", file.name));
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
                    linked: f.linked,
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
) -> Result<String, AppError> {
    let logger = TauriLogger { app: app.clone() };
    recover_registry_state(&app_id, &logger)?;

    let cfg = load_deployer_config(app, &deployer_id)?;
    match deployer_id.as_str() {
        "unreal_engine" => unreal::resolve_unreal_mod_path(&app_id, &cfg, &logger)
            .map(|p| p.to_string_lossy().to_string()),
        _ => Err(AppError::other(format!("Unsupported deployer: {}", deployer_id))),
    }
}

pub fn deploy_mod(
    app: &AppHandle,
    app_id: String,
    mod_id: String,
) -> Result<ModManifest, AppError> {
    let logger = TauriLogger { app: app.clone() };
    recover_registry_state(&app_id, &logger)?;

    let selected = get_selected_deployer(&app_id)?;
    let Some(deployer_id) = selected else {
        logger.error("No deployer configured");
        return Err(AppError::other("No deployer configured"));
    };

    if deployer_id == "NONE" {
        logger.error("No deployer configured");
        return Err(AppError::other("No deployer configured"));
    }

    ensure_game_not_running(&logger)?;

    let cfg = load_deployer_config(app, &deployer_id)?;
    let target_dir = match deployer_id.as_str() {
        "unreal_engine" => unreal::resolve_unreal_mod_path(&app_id, &cfg, &logger)?,
        _ => return Err(AppError::other(format!("Unsupported deployer: {}", deployer_id))),
    };

    if cfg.create_mod_folder {
        fs::create_dir_all(&target_dir).with_path(&target_dir)?;
    }

    logger.info(&format!("Deploying to {}", target_dir.display()));

    let source_dir = mod_files_dir(&app_id, &mod_id);
    let files = gather_mod_files(&source_dir, &cfg)?;
    logger.info(&format!("Found {} pak file(s)", files.len()));

    let mut deployed_files: Vec<DeployedFile> = Vec::new();
    let mut linked_count = 0usize;

    for src in files {
        let Some(name) = src.file_name().and_then(|n| n.to_str()) else {
            continue;
        };

        let target = target_dir.join(name);
        if target.exists() {
            logger.warning(&format!("Conflict: {} already exists", name));
            continue;
        }

        create_file_link(&src, &target, &logger)?;

        linked_count += 1;
        logger.success(&format!("Linked {}", name));
        deployed_files.push(DeployedFile {
            name: name.to_string(),
            target: target.to_string_lossy().to_string(),
            linked: true,
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
    logger.success(&format!(
        "Deployment complete — {} files linked",
        linked_count
    ));

    Ok(manifest)
}

pub fn undeploy_mod(
    app: &AppHandle,
    app_id: String,
    mod_id: String,
) -> Result<(), AppError> {
    let logger = TauriLogger { app: app.clone() };
    recover_registry_state(&app_id, &logger)?;

    ensure_game_not_running(&logger)?;

    let existing = load_manifest(&app_id, &mod_id)?;
    let Some(mut manifest) = existing else {
        remove_registry_mod(&app_id, &mod_id)?;
        logger.success("Mod removed");
        return Ok(());
    };

    for f in &mut manifest.files {
        let target = PathBuf::from(&f.target);
        if remove_link_if_present(&target)? {
            logger.info(&format!("Removed {}", f.name));
        }
        f.linked = false;
    }

    manifest.deployed = false;
    manifest.deployed_at = None;

    save_manifest(&app_id, &mod_id, &manifest)?;
    update_registry_from_manifest(&app_id, &mod_id, &manifest)?;
    logger.success("Mod removed");

    Ok(())
}

pub fn toggle_mod(
    app: &AppHandle,
    app_id: String,
    mod_id: String,
    enabled: bool,
) -> Result<(), AppError> {
    let logger = TauriLogger { app: app.clone() };
    recover_registry_state(&app_id, &logger)?;

    ensure_game_not_running(&logger)?;

    let existing = load_manifest(&app_id, &mod_id)?;
    let Some(mut manifest) = existing else {
        return Err(AppError::other("Mod manifest not found"));
    };

    if manifest.files.is_empty() {
        return Err(AppError::other("Mod manifest has no files"));
    }

    for f in &mut manifest.files {
        let target = PathBuf::from(&f.target);
        let source = source_file_path(&app_id, &mod_id, &f.name);

        if enabled {
            if !source.exists() {
                logger.error(&format!("Missing source file for {}", f.name));
                return Err(AppError::other(format!("Missing source file for {}", f.name)));
            }
            if target.symlink_metadata().is_err() {
                create_file_link(&source, &target, &logger)?;
            }
            f.linked = true;
        } else {
            remove_link_if_present(&target)?;
            f.linked = false;
        }
    }

    manifest.deployed = enabled;
    manifest.deployed_at = if enabled {
        manifest.deployed_at.or_else(rfc3339_now_utc)
    } else {
        None
    };

    save_manifest(&app_id, &mod_id, &manifest)?;
    update_registry_from_manifest(&app_id, &mod_id, &manifest)?;

    if enabled {
        logger.info(&format!("{} enabled", mod_id));
    } else {
        logger.info(&format!("{} disabled", mod_id));
    }

    Ok(())
}

pub fn set_deployer_override_path(app_id: &str, path: &Path) -> Result<(), AppError> {
    let mut cfg = read_game_cache_config(app_id)?;
    let obj = cfg
        .as_table_mut()
        .ok_or_else(|| AppError::other("Cache config root must be a TOML table"))?;
    obj.insert(
        "deployer_mod_path".to_string(),
        toml::Value::String(path.to_string_lossy().to_string()),
    );
    write_game_cache_config(app_id, &cfg)
}

pub fn patch_game_cache_config(
    app_id: &str,
    patch: HashMap<String, Value>,
) -> Result<(), AppError> {
    let mut cfg = read_game_cache_config(app_id)?;
    let obj = cfg
        .as_table_mut()
        .ok_or_else(|| AppError::other("Cache config root must be a TOML table"))?;

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
) -> Result<ModManifest, AppError> {
    let logger = TauriLogger { app: app.clone() };
    recover_registry_state(&app_id, &logger)?;

    let selected = get_selected_deployer(&app_id)?;
    let Some(deployer_id) = selected else {
        logger.error("No deployer configured");
        return Err(AppError::other("No deployer configured"));
    };
    if deployer_id == "NONE" {
        logger.error("No deployer configured");
        return Err(AppError::other("No deployer configured"));
    }

    let cfg = load_deployer_config(app, &deployer_id)?;
    let raw = crate::config::get_raw_modlist_listings(app_id.clone())?;
    let listings = annotate_modlist_with_deployer(app, &app_id, raw)?;
    let listing = listings
        .into_iter()
        .find(|l| l.mod_id == listing_id)
        .ok_or_else(|| AppError::other(format!("Listing not found: {}", listing_id)))?;

    if !listing.deployable {
        let reason = listing
            .deployer_reason
            .clone()
            .unwrap_or_else(|| "Listing is not accepted by selected deployer".to_string());
        logger.warning(&reason);
        return Err(AppError::other(reason));
    }

    ensure_game_not_running(&logger)?;

    let mod_id = listing.mod_id.clone();
    let files_dir = mod_files_dir(&app_id, &mod_id);
    fs::create_dir_all(&files_dir).with_path(&files_dir)?;

    let mut copied = 0usize;
    let mut seen = HashSet::new();
    let source_parent = listing.source_path.as_ref().and_then(|p| {
        let path = PathBuf::from(p);
        if path.is_dir() {
            Some(path)
        } else {
            path.parent().map(|p| p.to_path_buf())
        }
    });

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
        let target = files_dir.join(&file_name);
        fs::copy(&src, &target).with_path(&target)?;
        copied += 1;
    }

    if copied == 0 {
        return Err(AppError::other(
            "No deployable files were staged from listing",
        ));
    }

    logger.info(&format!("Staged {} file(s) for {}", copied, mod_id));
    let manifest = deploy_mod(app, app_id.clone(), mod_id.clone())?;
    crate::filehandler::runtime_reader::upsert_profile_from_deploy(
        &app_id,
        &deployer_id,
        &listing,
        &manifest,
    )?;
    logger.success(&format!("Profile updated for {}", mod_id));
    Ok(manifest)
}

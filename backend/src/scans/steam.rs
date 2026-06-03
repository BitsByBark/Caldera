use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use crate::{AppError, ArtworkPaths, SteamGame, WithPath};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ManualGameEntry {
    app_id: String,
    name: String,
    install_path: String,
    source: String,
}

#[derive(Debug, Clone)]
enum VdfValue {
    Str(String),
    Obj(HashMap<String, VdfValue>),
}

struct VdfParser<'a> {
    chars: Vec<char>,
    i: usize,
    _src: &'a str,
}

impl<'a> VdfParser<'a> {
    fn new(src: &'a str) -> Self {
        Self {
            chars: src.chars().collect(),
            i: 0,
            _src: src,
        }
    }

    fn parse(mut self) -> Result<HashMap<String, VdfValue>, String> {
        self.parse_object(false)
    }

    fn parse_object(&mut self, expect_brace: bool) -> Result<HashMap<String, VdfValue>, String> {
        let mut map = HashMap::new();

        if expect_brace {
            self.consume_whitespace();
            if self.peek() != Some('{') {
                return Err("expected '{'".to_string());
            }
            self.i += 1;
        }

        loop {
            self.consume_whitespace();
            match self.peek() {
                None => break,
                Some('}') => {
                    self.i += 1;
                    break;
                }
                _ => {
                    let key = self.parse_string()?;
                    self.consume_whitespace();
                    let value = match self.peek() {
                        Some('{') => VdfValue::Obj(self.parse_object(true)?),
                        _ => VdfValue::Str(self.parse_string()?),
                    };
                    map.insert(key, value);
                }
            }
        }

        Ok(map)
    }

    fn parse_string(&mut self) -> Result<String, String> {
        self.consume_whitespace();
        if self.peek() != Some('"') {
            return Err("expected quote".to_string());
        }
        self.i += 1;
        let mut out = String::new();

        while let Some(c) = self.peek() {
            self.i += 1;
            if c == '"' {
                return Ok(out);
            }
            if c == '\\' {
                if let Some(next) = self.peek() {
                    self.i += 1;
                    out.push(match next {
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        '\\' => '\\',
                        '"' => '"',
                        other => other,
                    });
                }
            } else {
                out.push(c);
            }
        }

        Err("unterminated string".to_string())
    }

    fn consume_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.i += 1;
            } else {
                break;
            }
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.i).copied()
    }
}

fn parse_vdf(text: &str) -> Result<HashMap<String, VdfValue>, String> {
    VdfParser::new(text).parse()
}

fn get_obj<'a>(
    map: &'a HashMap<String, VdfValue>,
    key: &str,
) -> Option<&'a HashMap<String, VdfValue>> {
    match map.get(key) {
        Some(VdfValue::Obj(obj)) => Some(obj),
        _ => None,
    }
}

fn get_str<'a>(map: &'a HashMap<String, VdfValue>, key: &str) -> Option<&'a str> {
    match map.get(key) {
        Some(VdfValue::Str(s)) => Some(s.as_str()),
        _ => None,
    }
}

fn normalize_path(path: &str) -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        PathBuf::from(path.replace("\\\\", "\\"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        PathBuf::from(path)
    }
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

fn manual_games_path() -> PathBuf {
    base_config_dir().join("manual_games.json")
}

fn normalize_name(name: &str) -> String {
    name.trim().to_ascii_lowercase()
}

fn normalize_install_path(path: &Path) -> String {
    path.to_string_lossy().trim().to_ascii_lowercase()
}

fn manual_app_id(name: &str) -> String {
    format!("manual:{}", slugify_name(name))
}

fn copy_dir_missing(src: &Path, dest: &Path) -> Result<(), AppError> {
    if !src.exists() || !src.is_dir() {
        return Ok(());
    }
    fs::create_dir_all(dest).with_path(dest)?;
    for entry in fs::read_dir(src).with_path(src)? {
        let entry = entry.with_path(src)?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_missing(&src_path, &dest_path)?;
        } else if src_path.is_file() && !dest_path.exists() {
            fs::copy(&src_path, &dest_path).with_path(&dest_path)?;
        }
    }
    Ok(())
}

fn import_legacy_downloads(game: &SteamGame, game_root: &Path) -> Result<(), AppError> {
    let legacy_dir = base_config_dir()
        .join("downloads")
        .join(format!("{}-{}", slugify_name(&game.name), game.app_id));
    if !legacy_dir.exists() || !legacy_dir.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(&legacy_dir).with_path(&legacy_dir)? {
        let entry = entry.with_path(&legacy_dir)?;
        let src_path = entry.path();
        if !src_path.is_file() {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy().to_string();
        if file_name.ends_with(".json") || file_name.ends_with(".part") {
            continue;
        }
        let mod_id = format!(
            "legacy-{}",
            crate::filehandler::runtime_reader::safe_runtime_name(
                src_path
                    .file_stem()
                    .and_then(|name| name.to_str())
                    .unwrap_or("download"),
                "download",
            )
        );
        let mod_root = game_root.join("mods").join(&mod_id);
        let files_dir = mod_root.join("files");
        fs::create_dir_all(&files_dir).with_path(&files_dir)?;
        let dest_path = files_dir.join(&file_name);
        if !dest_path.exists() {
            fs::copy(&src_path, &dest_path).with_path(&dest_path)?;
        }
        let meta_path = mod_root.join("meta.toml");
        if meta_path.exists() {
            continue;
        }
        let listing = crate::config::ModListing {
            mod_id: mod_id.clone(),
            name: file_name.clone(),
            status: "downloaded".to_string(),
            source_path: Some(dest_path.to_string_lossy().to_string()),
            deployable: false,
            deployer_reason: None,
            added_at: None,
            progress: Some(1.0),
            speed: Some("0 KB/S".to_string()),
            version: None,
            author: None,
            description: None,
            summary: None,
            source: Some("legacy-download".to_string()),
            source_url: None,
            nexus_mod_id: None,
            nexus_file_id: None,
            categories: Vec::new(),
            tags: Vec::new(),
            file_size: fs::metadata(&dest_path).ok().map(|m| m.len() as f64),
            file_count: Some(1.0),
            file_types: Vec::new(),
            user_notes: None,
            favorite: None,
            files: vec![file_name],
        };
        let body = toml::to_string_pretty(&listing).map_err(AppError::TomlSerialize)?;
        fs::write(&meta_path, body).with_path(&meta_path)?;
    }
    Ok(())
}

fn ensure_game_runtime_dirs(game: &SteamGame) -> Result<(), AppError> {
    let old_root = crate::filehandler::runtime_reader::library_dir().join(&game.app_id);
    let new_root = crate::filehandler::runtime_reader::game_dir_for_name(&game.app_id, &game.name);
    crate::filehandler::runtime_reader::ensure_game_dirs_for_name(&game.app_id, &game.name)?;
    if old_root != new_root {
        copy_dir_missing(&old_root, &new_root)?;
    }
    import_legacy_downloads(game, &new_root)?;

    let config_toml = new_root.join("metadata").join("config.toml");
    if !config_toml.exists() {
        fs::write(&config_toml, "").with_path(&config_toml)?;
    }

    let meta = json!({
        "app_id": game.app_id,
        "name": game.name,
        "install_path": game.install_path
    });
    let meta_path = new_root.join("metadata").join("meta.json");
    fs::write(
        &meta_path,
        serde_json::to_string_pretty(&meta).unwrap_or_else(|_| "{}".to_string()),
    )
    .with_path(&meta_path)?;

    Ok(())
}

fn load_manual_games() -> Result<Vec<SteamGame>, AppError> {
    let path = manual_games_path();
    if !path.exists() {
        return Ok(Vec::new());
    }

    let raw = fs::read_to_string(&path).with_path(&path)?;
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }

    let parsed: Vec<ManualGameEntry> = serde_json::from_str(&raw).map_err(AppError::Json)?;

    Ok(parsed
        .into_iter()
        .map(|m| SteamGame {
            app_id: m.app_id,
            name: m.name,
            install_path: m.install_path,
        })
        .collect())
}

fn save_manual_games(games: &[SteamGame]) -> Result<(), AppError> {
    let path = manual_games_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_path(parent)?;
    }

    let entries: Vec<ManualGameEntry> = games
        .iter()
        .map(|g| ManualGameEntry {
            app_id: g.app_id.clone(),
            name: g.name.clone(),
            install_path: g.install_path.clone(),
            source: "manual".to_string(),
        })
        .collect();

    let body = serde_json::to_string_pretty(&entries).map_err(AppError::Json)?;
    fs::write(&path, body).with_path(&path)
}

#[cfg(target_os = "linux")]
fn default_steam_root() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    let p = PathBuf::from(home).join(".steam").join("steam");
    if p.exists() {
        Some(p)
    } else {
        None
    }
}

#[cfg(target_os = "windows")]
fn default_steam_root() -> Option<PathBuf> {
    // TODO: Use `winreg` crate to read:
    // HKEY_CURRENT_USER\Software\Valve\Steam -> InstallPath
    None
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn default_steam_root() -> Option<PathBuf> {
    None
}

fn resolve_steam_root(steam_path: Option<String>) -> Option<PathBuf> {
    if let Some(raw) = steam_path {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            let p = PathBuf::from(trimmed);
            if p.exists() {
                return Some(p);
            }
        }
    }
    default_steam_root()
}

#[cfg(target_os = "windows")]
fn steam_path_required_message() -> String {
    "Steam path is required on Windows. Set it in Settings > steam_path.".to_string()
}

#[cfg(not(target_os = "windows"))]
fn steam_path_required_message() -> String {
    "Steam installation not found. Set your Steam path in Settings.".to_string()
}

fn library_steamapps_dirs(steam_root: &Path) -> Vec<PathBuf> {
    let primary = steam_root.join("steamapps");
    let mut libs: BTreeSet<PathBuf> = BTreeSet::new();

    if primary.exists() {
        libs.insert(primary.clone());
    }

    let vdf_path = primary.join("libraryfolders.vdf");
    let Ok(vdf_text) = fs::read_to_string(vdf_path) else {
        return libs.into_iter().collect();
    };

    let Ok(doc) = parse_vdf(&vdf_text) else {
        return libs.into_iter().collect();
    };

    let Some(libfolders) = get_obj(&doc, "libraryfolders") else {
        return libs.into_iter().collect();
    };

    for (key, val) in libfolders {
        if key.parse::<u32>().is_err() {
            continue;
        }

        match val {
            VdfValue::Str(path) => {
                let steamapps = normalize_path(path).join("steamapps");
                if steamapps.exists() {
                    libs.insert(steamapps);
                }
            }
            VdfValue::Obj(obj) => {
                if let Some(path) = get_str(obj, "path") {
                    let steamapps = normalize_path(path).join("steamapps");
                    if steamapps.exists() {
                        libs.insert(steamapps);
                    }
                }
            }
        }
    }

    libs.into_iter().collect()
}

fn parse_appmanifest(path: &Path, library_steamapps: &Path) -> Option<SteamGame> {
    let text = fs::read_to_string(path).ok()?;
    let doc = parse_vdf(&text).ok()?;
    let app_state = get_obj(&doc, "AppState")?;

    let appid = get_str(app_state, "appid")?.to_string();
    let name = get_str(app_state, "name")?.to_string();
    let installdir = get_str(app_state, "installdir")?.to_string();

    let install_path = library_steamapps.join("common").join(installdir);

    Some(SteamGame {
        app_id: appid,
        name,
        install_path: install_path.to_string_lossy().to_string(),
    })
}

pub fn get_steam_games(steam_path: Option<String>) -> Result<Vec<SteamGame>, AppError> {
    let mut games = Vec::new();
    let steam_scan_failed = if let Some(steam_root) = resolve_steam_root(steam_path) {
        let libs = library_steamapps_dirs(&steam_root);
        for lib in libs {
            let Ok(entries) = fs::read_dir(&lib) else {
                continue;
            };

            for entry in entries.flatten() {
                let p = entry.path();
                let is_acf = p.extension().and_then(|e| e.to_str()) == Some("acf");
                let is_manifest = p
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("appmanifest_"))
                    .unwrap_or(false);

                if !(is_acf && is_manifest) {
                    continue;
                }

                if let Some(game) = parse_appmanifest(&p, &lib) {
                    games.push(game);
                }
            }
        }
        false
    } else {
        true
    };

    let manual_games = load_manual_games()?;
    games.extend(manual_games);

    games.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    games.dedup_by(|a, b| a.app_id == b.app_id);

    if games.is_empty() && steam_scan_failed {
        return Err(AppError::other(steam_path_required_message()));
    }

    // Keep per-game runtime roots in sync with discovered/manual games.
    for game in &games {
        let _ = ensure_game_runtime_dirs(game);
    }

    Ok(games)
}

pub fn add_manual_game(name: String, install_path: String) -> Result<SteamGame, AppError> {
    let trimmed_name = name.trim();
    if trimmed_name.is_empty() {
        return Err(AppError::other("Game name is required"));
    }

    let resolved_install_path = {
        let trimmed_path = install_path.trim();
        if trimmed_path.is_empty() {
            String::new()
        } else {
            let install_path_buf = PathBuf::from(trimmed_path);
            if !install_path_buf.exists() || !install_path_buf.is_dir() {
                return Err(AppError::other("Install directory does not exist"));
            }
            install_path_buf.to_string_lossy().to_string()
        }
    };

    let mut manual_games = load_manual_games()?;
    let norm_name = normalize_name(trimmed_name);

    let duplicate = if resolved_install_path.is_empty() {
        manual_games
            .iter()
            .any(|g| normalize_name(&g.name) == norm_name && g.install_path.trim().is_empty())
    } else {
        let norm_path = normalize_install_path(Path::new(&resolved_install_path));
        manual_games.iter().any(|g| {
            normalize_name(&g.name) == norm_name
                && normalize_install_path(Path::new(&g.install_path)) == norm_path
        })
    };
    if duplicate {
        return Err(AppError::other(if resolved_install_path.is_empty() {
            "Game with same name already exists"
        } else {
            "Game with same name and install directory already exists"
        }));
    }

    let base_id = manual_app_id(trimmed_name);
    let mut next_id = base_id.clone();
    let mut suffix = 2usize;
    let existing_ids: std::collections::HashSet<String> =
        manual_games.iter().map(|g| g.app_id.clone()).collect();
    while existing_ids.contains(&next_id) {
        next_id = format!("{}-{}", base_id, suffix);
        suffix += 1;
    }

    let game = SteamGame {
        app_id: next_id,
        name: trimmed_name.to_string(),
        install_path: resolved_install_path,
    };
    manual_games.push(game.clone());
    save_manual_games(&manual_games)?;
    ensure_game_runtime_dirs(&game)?;

    Ok(game)
}

pub fn get_game_artwork(app_id: String, steam_path: Option<String>) -> ArtworkPaths {
    let caldera_artwork = crate::filehandler::runtime_reader::artwork_dir(&app_id);
    let cached_banner = caldera_artwork.join("banner.jpg");
    let cached_hero = caldera_artwork.join("hero.jpg");
    let cached_logo = caldera_artwork.join("logo.png");

    let steam_cache =
        resolve_steam_root(steam_path).map(|root| root.join("appcache").join("librarycache"));

    let steam_banner = steam_cache
        .as_ref()
        .map(|cache| cache.join(format!("{}_library_600x900.jpg", app_id)));
    let steam_hero = steam_cache
        .as_ref()
        .map(|cache| cache.join(format!("{}_library_hero.jpg", app_id)));
    let steam_logo = steam_cache
        .as_ref()
        .map(|cache| cache.join(format!("{}_logo.png", app_id)));

    let banner = if cached_banner.exists() {
        cached_banner
    } else {
        steam_banner.unwrap_or_default()
    };
    let hero = if cached_hero.exists() {
        cached_hero
    } else {
        steam_hero.unwrap_or_default()
    };
    let logo = if cached_logo.exists() {
        cached_logo
    } else {
        steam_logo.unwrap_or_default()
    };

    ArtworkPaths {
        banner: if banner.exists() {
            banner.to_string_lossy().to_string()
        } else {
            String::new()
        },
        hero: if hero.exists() {
            hero.to_string_lossy().to_string()
        } else {
            String::new()
        },
        logo: if logo.exists() {
            logo.to_string_lossy().to_string()
        } else {
            String::new()
        },
    }
}

fn base_config_dir() -> PathBuf {
    crate::filehandler::runtime::base_config_dir()
}

fn find_librarycache_file(steam_root: &Path, app_id: &str, filename: &str) -> Option<PathBuf> {
    let app_dir = steam_root
        .join("appcache")
        .join("librarycache")
        .join(app_id);

    let entries = fs::read_dir(&app_dir).ok()?;
    for entry in entries.flatten() {
        let p = entry.path();
        if !p.is_dir() {
            continue;
        }
        let candidate = p.join(filename);
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

fn copy_if_missing(src: &Path, dest: &Path) -> Result<(), AppError> {
    if !src.exists() || dest.exists() {
        return Ok(());
    }
    fs::copy(src, dest).with_path(dest).map(|_| ())
}

pub fn ensure_game_cache(app_id: String, steam_path: Option<String>) -> Result<(), AppError> {
    let is_manual = app_id.starts_with("manual:");
    let steam_root = if is_manual {
        None
    } else {
        Some(
            resolve_steam_root(steam_path.clone())
                .ok_or_else(|| AppError::other(steam_path_required_message()))?,
        )
    };

    let game_meta = get_steam_games(steam_path)?
        .into_iter()
        .find(|g| g.app_id == app_id);

    let game = game_meta.unwrap_or_else(|| SteamGame {
        app_id: app_id.clone(),
        name: String::new(),
        install_path: String::new(),
    });
    ensure_game_runtime_dirs(&game)?;

    let artwork_dir = crate::filehandler::runtime_reader::artwork_dir(&app_id);

    if let Some(steam_root) = steam_root {
        if let Some(src) = find_librarycache_file(&steam_root, &app_id, "library_capsule.jpg") {
            copy_if_missing(&src, &artwork_dir.join("banner.jpg"))?;
        }
        if let Some(src) = find_librarycache_file(&steam_root, &app_id, "library_hero.jpg") {
            copy_if_missing(&src, &artwork_dir.join("hero.jpg"))?;
        }
        if let Some(src) = find_librarycache_file(&steam_root, &app_id, "logo.png") {
            copy_if_missing(&src, &artwork_dir.join("logo.png"))?;
        }
    }

    Ok(())
}

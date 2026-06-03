use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use crate::{ArtworkPaths, SteamGame};
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

fn ensure_game_runtime_dirs(game: &SteamGame) -> Result<(), String> {
    let cache_root = base_config_dir().join("cache").join(&game.app_id);
    let artwork_dir = cache_root.join("artwork");
    let mods_dir = cache_root.join("mods");
    let profiles_dir = cache_root.join("profiles");
    let downloads_root = base_config_dir().join("downloads");
    let downloads_dir =
        downloads_root.join(format!("{}-{}", slugify_name(&game.name), game.app_id));
    let metadata_dir = base_config_dir().join("metadata").join(format!(
        "{}-{}",
        slugify_name(&game.name),
        game.app_id
    ));

    fs::create_dir_all(&artwork_dir)
        .map_err(|e| format!("failed creating cache artwork dir: {}", e))?;
    fs::create_dir_all(&mods_dir).map_err(|e| format!("failed creating cache mods dir: {}", e))?;
    fs::create_dir_all(&profiles_dir)
        .map_err(|e| format!("failed creating cache profiles dir: {}", e))?;
    fs::create_dir_all(&downloads_root)
        .map_err(|e| format!("failed creating downloads dir: {}", e))?;
    fs::create_dir_all(&downloads_dir)
        .map_err(|e| format!("failed creating game downloads dir: {}", e))?;
    fs::create_dir_all(&metadata_dir)
        .map_err(|e| format!("failed creating metadata dir: {}", e))?;

    let config_toml = cache_root.join("config.toml");
    if !config_toml.exists() {
        fs::write(&config_toml, "").map_err(|e| format!("failed writing config.toml: {}", e))?;
    }

    let meta = json!({
        "app_id": game.app_id,
        "name": game.name,
        "install_path": game.install_path
    });
    let meta_path = cache_root.join("meta.json");
    fs::write(
        &meta_path,
        serde_json::to_string_pretty(&meta).unwrap_or_else(|_| "{}".to_string()),
    )
    .map_err(|e| format!("failed writing meta.json: {}", e))?;

    Ok(())
}

fn load_manual_games() -> Result<Vec<SteamGame>, String> {
    let path = manual_games_path();
    if !path.exists() {
        return Ok(Vec::new());
    }

    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("Failed reading manual games {}: {}", path.display(), e))?;
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }

    let parsed: Vec<ManualGameEntry> = serde_json::from_str(&raw)
        .map_err(|e| format!("Invalid manual games JSON {}: {}", path.display(), e))?;

    Ok(parsed
        .into_iter()
        .map(|m| SteamGame {
            app_id: m.app_id,
            name: m.name,
            install_path: m.install_path,
        })
        .collect())
}

fn save_manual_games(games: &[SteamGame]) -> Result<(), String> {
    let path = manual_games_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "Failed creating manual games dir {}: {}",
                parent.display(),
                e
            )
        })?;
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

    let body = serde_json::to_string_pretty(&entries)
        .map_err(|e| format!("Failed serializing manual games JSON: {}", e))?;
    fs::write(&path, body)
        .map_err(|e| format!("Failed writing manual games {}: {}", path.display(), e))
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

pub fn get_steam_games(steam_path: Option<String>) -> Result<Vec<SteamGame>, String> {
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
        return Err(steam_path_required_message());
    }

    // Keep per-game runtime roots in sync with discovered/manual games.
    for game in &games {
        let _ = ensure_game_runtime_dirs(game);
    }

    Ok(games)
}

pub fn add_manual_game(name: String, install_path: String) -> Result<SteamGame, String> {
    let trimmed_name = name.trim();
    if trimmed_name.is_empty() {
        return Err("Game name is required".to_string());
    }

    let resolved_install_path = {
        let trimmed_path = install_path.trim();
        if trimmed_path.is_empty() {
            String::new()
        } else {
            let install_path_buf = PathBuf::from(trimmed_path);
            if !install_path_buf.exists() || !install_path_buf.is_dir() {
                return Err("Install directory does not exist".to_string());
            }
            install_path_buf.to_string_lossy().to_string()
        }
    };

    let mut manual_games = load_manual_games()?;
    let norm_name = normalize_name(trimmed_name);

    let duplicate = if resolved_install_path.is_empty() {
        manual_games.iter().any(|g| {
            normalize_name(&g.name) == norm_name && g.install_path.trim().is_empty()
        })
    } else {
        let norm_path = normalize_install_path(Path::new(&resolved_install_path));
        manual_games.iter().any(|g| {
            normalize_name(&g.name) == norm_name
                && normalize_install_path(Path::new(&g.install_path)) == norm_path
        })
    };
    if duplicate {
        return Err(if resolved_install_path.is_empty() {
            "Game with same name already exists".to_string()
        } else {
            "Game with same name and install directory already exists".to_string()
        });
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
    let caldera_artwork = base_config_dir()
        .join("cache")
        .join(&app_id)
        .join("artwork");
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
    crate::runtime::base_config_dir()
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

fn copy_if_missing(src: &Path, dest: &Path) -> Result<(), String> {
    if !src.exists() || dest.exists() {
        return Ok(());
    }
    fs::copy(src, dest)
        .map(|_| ())
        .map_err(|e| format!("failed copying {}: {}", src.display(), e))
}

pub fn ensure_game_cache(app_id: String, steam_path: Option<String>) -> Result<(), String> {
    let is_manual = app_id.starts_with("manual:");
    let steam_root = if is_manual {
        None
    } else {
        Some(resolve_steam_root(steam_path.clone()).ok_or_else(|| steam_path_required_message())?)
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

    let cache_root = base_config_dir().join("cache").join(&app_id);
    let artwork_dir = cache_root.join("artwork");

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

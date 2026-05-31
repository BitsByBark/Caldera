use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use crate::{ArtworkPaths, SteamGame};
use serde_json::json;

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
        Self { chars: src.chars().collect(), i: 0, _src: src }
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

fn get_obj<'a>(map: &'a HashMap<String, VdfValue>, key: &str) -> Option<&'a HashMap<String, VdfValue>> {
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

#[cfg(target_os = "linux")]
fn default_steam_root() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    let p = PathBuf::from(home).join(".steam").join("steam");
    if p.exists() { Some(p) } else { None }
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
    let steam_root = resolve_steam_root(steam_path).ok_or_else(|| {
        "Steam installation not found. Set your Steam path in Settings.".to_string()
    })?;

    let libs = library_steamapps_dirs(&steam_root);
    if libs.is_empty() {
        return Ok(Vec::new());
    }

    let mut games = Vec::new();

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

    games.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    games.dedup_by(|a, b| a.app_id == b.app_id);

    Ok(games)
}

pub fn get_game_artwork(app_id: String, steam_path: Option<String>) -> ArtworkPaths {
    let Some(root) = resolve_steam_root(steam_path) else {
        return ArtworkPaths {
            banner: String::new(),
            hero: String::new(),
            logo: String::new(),
        };
    };

    let cache = root.join("appcache").join("librarycache");
    let banner = cache.join(format!("{}_library_600x900.jpg", app_id));
    let hero = cache.join(format!("{}_library_hero.jpg", app_id));
    let logo = cache.join(format!("{}_logo.png", app_id));

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
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("caldera")
}

fn appcache_library_path(steam_root: &Path, app_id: &str, suffix: &str) -> PathBuf {
    steam_root
        .join("appcache")
        .join("librarycache")
        .join(format!("{}_{}", app_id, suffix))
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
    let steam_root = resolve_steam_root(steam_path.clone()).ok_or_else(|| {
        "Steam installation not found. Set your Steam path in Settings.".to_string()
    })?;

    let cache_root = base_config_dir().join("cache").join(&app_id);
    let artwork_dir = cache_root.join("artwork");
    let mods_dir = cache_root.join("mods");
    let profiles_dir = cache_root.join("profiles");

    fs::create_dir_all(&artwork_dir)
        .map_err(|e| format!("failed creating cache artwork dir: {}", e))?;
    fs::create_dir_all(&mods_dir).map_err(|e| format!("failed creating cache mods dir: {}", e))?;
    fs::create_dir_all(&profiles_dir)
        .map_err(|e| format!("failed creating cache profiles dir: {}", e))?;

    let config_toml = cache_root.join("config.toml");
    if !config_toml.exists() {
        fs::write(&config_toml, "").map_err(|e| format!("failed writing config.toml: {}", e))?;
    }

    let banner_src = appcache_library_path(&steam_root, &app_id, "library_600x900.jpg");
    let hero_src = appcache_library_path(&steam_root, &app_id, "library_hero.jpg");
    let logo_src = appcache_library_path(&steam_root, &app_id, "logo.png");

    copy_if_missing(&banner_src, &artwork_dir.join("banner.jpg"))?;
    copy_if_missing(&hero_src, &artwork_dir.join("hero.jpg"))?;
    copy_if_missing(&logo_src, &artwork_dir.join("logo.png"))?;

    let game_meta = get_steam_games(steam_path)?
        .into_iter()
        .find(|g| g.app_id == app_id);

    let meta = match game_meta {
        Some(game) => json!({
            "app_id": game.app_id,
            "name": game.name,
            "install_path": game.install_path
        }),
        None => json!({
            "app_id": app_id,
            "name": "",
            "install_path": ""
        }),
    };

    let meta_path = cache_root.join("meta.json");
    fs::write(&meta_path, serde_json::to_string_pretty(&meta).unwrap_or_else(|_| "{}".to_string()))
        .map_err(|e| format!("failed writing meta.json: {}", e))?;

    Ok(())
}

use serde::{Deserialize, Serialize};

pub mod config;
pub mod deployer;
pub mod download;
pub mod nxm;
pub mod operations;
pub mod packer;
pub mod profile_format;
pub mod profile_runtime;
pub mod runtime;
pub mod steam;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteamGame {
    pub app_id: String,
    pub name: String,
    pub install_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtworkPaths {
    pub banner: String,
    pub hero: String,
    pub logo: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameConfig {
    pub game_id: String,
    pub name: String,
    pub mod_directory: String,
    pub deployer: Option<String>,
    pub active_profile: Option<String>,
    pub profiles: Vec<String>,
}

pub fn get_steam_games(steam_path: Option<String>) -> Result<Vec<SteamGame>, String> {
    steam::get_steam_games(steam_path)
}

pub fn add_manual_game(name: String, install_path: String) -> Result<SteamGame, String> {
    steam::add_manual_game(name, install_path)
}

pub fn set_working_directory(path: Option<String>) -> Result<String, String> {
    runtime::set_working_directory(path)
}

pub fn get_settings_schema() -> Result<String, String> {
    runtime::get_settings_schema()
}

pub fn get_settings_values() -> Result<serde_json::Value, String> {
    runtime::get_settings_values()
}

pub fn save_settings_values(values: serde_json::Value) -> Result<(), String> {
    runtime::save_settings_values(values)
}

pub fn get_game_artwork(app_id: String, steam_path: Option<String>) -> ArtworkPaths {
    steam::get_game_artwork(app_id, steam_path)
}

pub fn ensure_game_cache(app_id: String, steam_path: Option<String>) -> Result<(), String> {
    steam::ensure_game_cache(app_id, steam_path)
}

pub fn get_game_config(game_id: String) -> GameConfig {
    config::get_game_config_stub(game_id)
}

pub fn save_game_config(game_id: String, config: GameConfig) {
    config::save_game_config_stub(game_id, config)
}

pub fn export_pack(
    app: &tauri::AppHandle,
    app_id: String,
    profile_name: String,
    pack_name: String,
    pack_type: String,
    export_path: String,
    include_disabled: bool,
) -> Result<String, String> {
    packer::export::export_pack(
        app,
        app_id,
        profile_name,
        pack_name,
        pack_type,
        export_path,
        include_disabled,
    )
}

pub fn import_pack(
    app: &tauri::AppHandle,
    pack_path: String,
) -> Result<packer::ImportResult, String> {
    packer::import::import_pack(app, pack_path)
}

pub fn get_modlist_listings(
    app: &tauri::AppHandle,
    app_id: String,
) -> Result<Vec<config::ModListing>, String> {
    let raw = config::get_raw_modlist_listings(app_id.clone())?;
    deployer::annotate_modlist_with_deployer(app, &app_id, raw)
}

pub fn get_profile_modlist(app_id: String) -> Result<Vec<config::ProfileModRow>, String> {
    config::get_profile_modlist(app_id)
}

pub fn resolve_deployer_path(
    app: &tauri::AppHandle,
    app_id: String,
    deployer_id: String,
) -> Result<String, String> {
    deployer::resolve_deployer_path(app, app_id, deployer_id)
}

pub fn deploy_mod(
    app: &tauri::AppHandle,
    app_id: String,
    mod_id: String,
) -> Result<deployer::ModManifest, String> {
    deployer::deploy_mod(app, app_id, mod_id)
}

pub fn undeploy_mod(app: &tauri::AppHandle, app_id: String, mod_id: String) -> Result<(), String> {
    deployer::undeploy_mod(app, app_id, mod_id)
}

pub fn toggle_mod(
    app: &tauri::AppHandle,
    app_id: String,
    mod_id: String,
    enabled: bool,
) -> Result<(), String> {
    deployer::toggle_mod(app, app_id, mod_id, enabled)
}

pub fn toggle_profile_mod(
    app: &tauri::AppHandle,
    app_id: String,
    mod_id: String,
    enabled: bool,
) -> Result<(), String> {
    if !config::profile_contains_mod(&app_id, &mod_id)? {
        return Err(format!("Mod {} is not in the active profile", mod_id));
    }
    deployer::toggle_mod(app, app_id, mod_id, enabled)
}

pub fn get_available_deployers(
    app: &tauri::AppHandle,
) -> Result<Vec<deployer::DeployerOption>, String> {
    deployer::get_available_deployers(app)
}

pub fn get_configured_deployer(app_id: String) -> Result<Option<String>, String> {
    deployer::get_configured_deployer(&app_id)
}

pub fn set_game_deployer(
    app: &tauri::AppHandle,
    app_id: String,
    deployer_id: String,
) -> Result<(), String> {
    deployer::set_game_deployer(app, &app_id, &deployer_id)
}

pub fn uncompress_archive(archive_path: String) -> Result<Vec<String>, String> {
    operations::uncompress::uncompress(archive_path)
}

pub fn deploy_listing(
    app: &tauri::AppHandle,
    app_id: String,
    listing_id: String,
) -> Result<deployer::ModManifest, String> {
    deployer::deploy_listing(app, app_id, listing_id)
}

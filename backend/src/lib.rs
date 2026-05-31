use serde::{Deserialize, Serialize};

pub mod config;
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameConfig {
    pub game_id: String,
    pub name: String,
    pub mod_directory: String,
    pub profiles: Vec<String>,
}

pub fn get_steam_games(steam_path: Option<String>) -> Result<Vec<SteamGame>, String> {
    steam::get_steam_games(steam_path)
}

pub fn get_game_artwork(app_id: String, steam_path: Option<String>) -> ArtworkPaths {
    steam::get_game_artwork(app_id, steam_path)
}

pub fn get_game_config(game_id: String) -> GameConfig {
    config::get_game_config_stub(game_id)
}

pub fn save_game_config(game_id: String, config: GameConfig) {
    config::save_game_config_stub(game_id, config)
}

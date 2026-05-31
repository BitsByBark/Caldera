use std::path::PathBuf;

use crate::GameConfig;

fn base_config_dir() -> PathBuf {
    // Linux: ~/.config/caldera/
    // Windows: %APPDATA%\\caldera\\
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("caldera")
}

fn game_config_path(game_id: &str) -> PathBuf {
    base_config_dir().join("games").join(format!("{}.toml", game_id))
}

pub fn get_game_config_stub(game_id: String) -> GameConfig {
    let _path = game_config_path(&game_id);
    GameConfig {
        game_id: game_id.clone(),
        name: format!("Game {}", game_id),
        mod_directory: "~/CALDERA/mods".to_string(),
        profiles: vec!["default".to_string()],
    }
}

pub fn save_game_config_stub(game_id: String, config: GameConfig) {
    let _path = game_config_path(&game_id);
    let _serialized = toml::to_string(&config).unwrap_or_default();
}

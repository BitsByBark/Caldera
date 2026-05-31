use caldera_backend::{ArtworkPaths, GameConfig, SteamGame};

#[tauri::command(rename_all = "camelCase")]
fn get_steam_games(steam_path: Option<String>) -> Result<Vec<SteamGame>, String> {
    caldera_backend::get_steam_games(steam_path)
}

#[tauri::command(rename_all = "camelCase")]
fn get_game_artwork(app_id: String, steam_path: Option<String>) -> ArtworkPaths {
    caldera_backend::get_game_artwork(app_id, steam_path)
}

#[tauri::command(rename_all = "camelCase")]
fn get_game_config(game_id: String) -> GameConfig {
    caldera_backend::get_game_config(game_id)
}

#[tauri::command(rename_all = "camelCase")]
fn save_game_config(game_id: String, config: GameConfig) {
    caldera_backend::save_game_config(game_id, config)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            get_steam_games,
            get_game_artwork,
            get_game_config,
            save_game_config
        ])
        .run(tauri::generate_context!())
        .expect("failed to run CALDERA Tauri app");
}

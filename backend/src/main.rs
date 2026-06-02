use caldera_backend::{deployer::{DeployerOption, ModManifest}, ArtworkPaths, GameConfig, SteamGame};
use tauri::Emitter;

#[derive(Clone, serde::Serialize)]
struct OperationProgressEvent {
    operation: String,
    target: String,
    progress: u8,
    message: String,
}

#[tauri::command(rename_all = "camelCase")]
fn get_steam_games(steam_path: Option<String>) -> Result<Vec<SteamGame>, String> {
    caldera_backend::get_steam_games(steam_path)
}

#[tauri::command(rename_all = "camelCase")]
fn add_manual_game(name: String, install_path: String) -> Result<SteamGame, String> {
    caldera_backend::add_manual_game(name, install_path)
}

#[tauri::command(rename_all = "camelCase")]
fn set_working_directory(path: Option<String>) -> Result<String, String> {
    caldera_backend::set_working_directory(path)
}

#[tauri::command(rename_all = "camelCase")]
fn get_settings_schema() -> Result<String, String> {
    caldera_backend::get_settings_schema()
}

#[tauri::command(rename_all = "camelCase")]
fn get_settings_values() -> Result<serde_json::Value, String> {
    caldera_backend::get_settings_values()
}

#[tauri::command(rename_all = "camelCase")]
fn save_settings_values(values: serde_json::Value) -> Result<(), String> {
    caldera_backend::save_settings_values(values)
}

#[tauri::command(rename_all = "camelCase")]
fn get_game_artwork(app_id: String, steam_path: Option<String>) -> ArtworkPaths {
    caldera_backend::get_game_artwork(app_id, steam_path)
}

#[tauri::command(rename_all = "camelCase")]
fn ensure_game_cache(app_id: String, steam_path: Option<String>) -> Result<(), String> {
    caldera_backend::ensure_game_cache(app_id, steam_path)
}

#[tauri::command(rename_all = "camelCase")]
fn get_game_config(game_id: String) -> GameConfig {
    caldera_backend::get_game_config(game_id)
}

#[tauri::command(rename_all = "camelCase")]
fn save_game_config(game_id: String, config: GameConfig) {
    caldera_backend::save_game_config(game_id, config)
}

#[tauri::command(rename_all = "camelCase")]
fn get_modlist_listings(app: tauri::AppHandle, app_id: String) -> Result<Vec<caldera_backend::config::ModListing>, String> {
    caldera_backend::get_modlist_listings(&app, app_id)
}

#[tauri::command(rename_all = "camelCase")]
fn get_profile_modlist(app_id: String) -> Result<Vec<caldera_backend::config::ProfileModRow>, String> {
    caldera_backend::get_profile_modlist(app_id)
}

#[tauri::command(rename_all = "camelCase")]
fn resolve_deployer_path(
    app: tauri::AppHandle,
    app_id: String,
    deployer_id: String,
) -> Result<String, String> {
    caldera_backend::resolve_deployer_path(&app, app_id, deployer_id)
}

#[tauri::command(rename_all = "camelCase")]
fn deploy_mod(app: tauri::AppHandle, app_id: String, mod_id: String) -> Result<ModManifest, String> {
    caldera_backend::deploy_mod(&app, app_id, mod_id)
}

#[tauri::command(rename_all = "camelCase")]
fn undeploy_mod(app: tauri::AppHandle, app_id: String, mod_id: String) -> Result<(), String> {
    caldera_backend::undeploy_mod(&app, app_id, mod_id)
}

#[tauri::command(rename_all = "camelCase")]
fn toggle_mod(
    app: tauri::AppHandle,
    app_id: String,
    mod_id: String,
    enabled: bool,
) -> Result<(), String> {
    caldera_backend::toggle_mod(&app, app_id, mod_id, enabled)
}

#[tauri::command(rename_all = "camelCase")]
fn toggle_profile_mod(
    app: tauri::AppHandle,
    app_id: String,
    mod_id: String,
    enabled: bool,
) -> Result<(), String> {
    caldera_backend::toggle_profile_mod(&app, app_id, mod_id, enabled)
}

#[tauri::command(rename_all = "camelCase")]
fn get_available_deployers(app: tauri::AppHandle) -> Result<Vec<DeployerOption>, String> {
    caldera_backend::get_available_deployers(&app)
}

#[tauri::command(rename_all = "camelCase")]
fn get_configured_deployer(app_id: String) -> Result<Option<String>, String> {
    caldera_backend::get_configured_deployer(app_id)
}

#[tauri::command(rename_all = "camelCase")]
fn set_game_deployer(
    app: tauri::AppHandle,
    app_id: String,
    deployer_id: String,
) -> Result<(), String> {
    caldera_backend::set_game_deployer(&app, app_id, deployer_id)
}

#[tauri::command(rename_all = "camelCase")]
fn uncompress_archive(app: tauri::AppHandle, archive_path: String) -> Result<Vec<String>, String> {
    let target = archive_path.clone();
    let emit = |progress: u8, message: String| {
        let _ = app.emit(
            "caldera://operation-progress",
            OperationProgressEvent {
                operation: "uncompress".to_string(),
                target: target.clone(),
                progress,
                message,
            },
        );
    };
    emit(0, format!("Uncompressing {}", archive_path));
    let result = caldera_backend::operations::uncompress::uncompress_with_progress(archive_path, |p| {
        emit(p, format!("Uncompressing... {}%", p));
    })?;
    emit(100, "Uncompress complete".to_string());
    Ok(result)
}

#[tauri::command(rename_all = "camelCase")]
fn deploy_listing(
    app: tauri::AppHandle,
    app_id: String,
    listing_id: String,
) -> Result<ModManifest, String> {
    caldera_backend::deploy_listing(&app, app_id, listing_id)
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let app = app.handle().clone();
            tauri::async_runtime::spawn(async {
                if let Err(err) = caldera_backend::download::run_server(app).await {
                    eprintln!("CALDERA download server error: {}", err);
                }
            });
            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_steam_games,
            add_manual_game,
            set_working_directory,
            get_settings_schema,
            get_settings_values,
            save_settings_values,
            get_game_artwork,
            ensure_game_cache,
            get_game_config,
            save_game_config,
            get_modlist_listings,
            get_profile_modlist,
            resolve_deployer_path,
            deploy_mod,
            undeploy_mod,
            toggle_mod,
            toggle_profile_mod,
            get_available_deployers,
            get_configured_deployer,
            set_game_deployer,
            uncompress_archive,
            deploy_listing
        ])
        .run(tauri::generate_context!())
        .expect("failed to run CALDERA Tauri app");
}

use caldera_backend::{
    deployer::{DeployerOption, ModManifest},
    ArtworkPaths, GameConfig, SteamGame,
};
use tauri::Emitter;
use tauri_plugin_deep_link::DeepLinkExt;

#[derive(Clone, serde::Serialize)]
struct OperationProgressEvent {
    operation: String,
    target: String,
    progress: u8,
    message: String,
}

#[tauri::command(rename_all = "camelCase")]
fn get_steam_games(steam_path: Option<String>) -> Result<Vec<SteamGame>, String> {
    caldera_backend::get_steam_games(steam_path).map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "camelCase")]
fn add_manual_game(name: String, install_path: String) -> Result<SteamGame, String> {
    caldera_backend::add_manual_game(name, install_path).map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "camelCase")]
fn set_working_directory(path: Option<String>) -> Result<String, String> {
    caldera_backend::set_working_directory(path).map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "camelCase")]
fn get_settings_schema() -> Result<String, String> {
    caldera_backend::get_settings_schema().map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "camelCase")]
fn get_settings_values() -> Result<serde_json::Value, String> {
    caldera_backend::get_settings_values().map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "camelCase")]
fn save_settings_values(values: serde_json::Value) -> Result<(), String> {
    caldera_backend::save_settings_values(values).map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "camelCase")]
fn get_game_artwork(app_id: String, steam_path: Option<String>) -> ArtworkPaths {
    caldera_backend::get_game_artwork(app_id, steam_path)
}

#[tauri::command(rename_all = "camelCase")]
fn ensure_game_cache(app_id: String, steam_path: Option<String>) -> Result<(), String> {
    caldera_backend::ensure_game_cache(app_id, steam_path).map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "camelCase")]
fn get_game_config(game_id: String) -> GameConfig {
    caldera_backend::get_game_config(game_id)
}

#[tauri::command(rename_all = "camelCase")]
fn save_game_config(game_id: String, config: GameConfig) -> Result<(), String> {
    caldera_backend::save_game_config(game_id, config).map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "camelCase")]
fn export_pack(
    app: tauri::AppHandle,
    app_id: String,
    profile_name: String,
    pack_name: String,
    version: String,
    pack_type: String,
    export_path: String,
    include_disabled: bool,
) -> Result<String, String> {
    caldera_backend::export_pack(
        &app,
        app_id,
        profile_name,
        pack_name,
        version,
        pack_type,
        export_path,
        include_disabled,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "camelCase")]
fn import_pack(
    app: tauri::AppHandle,
    pack_path: String,
) -> Result<caldera_backend::filehandler::packer::ImportResult, String> {
    caldera_backend::import_pack(&app, pack_path).map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "camelCase")]
fn list_collections(
    app_id: String,
) -> Result<Vec<caldera_backend::filehandler::packer::collections::CollectionEntry>, String> {
    caldera_backend::list_collections(app_id).map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "camelCase")]
async fn fetch_nexus_collections(app_id: String, game_domain: String) -> Result<(), String> {
    caldera_backend::fetch_nexus_collections(app_id, game_domain)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "camelCase")]
fn get_modlist_listings(
    app: tauri::AppHandle,
    app_id: String,
) -> Result<Vec<caldera_backend::config::ModListing>, String> {
    caldera_backend::get_modlist_listings(&app, app_id).map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "camelCase")]
fn open_mod_deploy_folder(app_id: String, mod_id: String) -> Result<String, String> {
    caldera_backend::open_mod_deploy_folder(app_id, mod_id).map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "camelCase")]
fn open_downloads_folder(app_id: String) -> Result<String, String> {
    caldera_backend::open_downloads_folder(app_id).map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "camelCase")]
fn get_profile_modlist(
    app_id: String,
) -> Result<Vec<caldera_backend::config::ProfileModRow>, String> {
    caldera_backend::get_profile_modlist(app_id).map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "camelCase")]
fn resolve_deployer_path(
    app: tauri::AppHandle,
    app_id: String,
    deployer_id: String,
) -> Result<String, String> {
    caldera_backend::resolve_deployer_path(&app, app_id, deployer_id).map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "camelCase")]
fn deploy_mod(
    app: tauri::AppHandle,
    app_id: String,
    mod_id: String,
) -> Result<ModManifest, String> {
    caldera_backend::deploy_mod(&app, app_id, mod_id).map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "camelCase")]
fn undeploy_mod(app: tauri::AppHandle, app_id: String, mod_id: String) -> Result<(), String> {
    caldera_backend::undeploy_mod(&app, app_id, mod_id).map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "camelCase")]
fn toggle_mod(
    app: tauri::AppHandle,
    app_id: String,
    mod_id: String,
    enabled: bool,
) -> Result<(), String> {
    caldera_backend::toggle_mod(&app, app_id, mod_id, enabled).map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "camelCase")]
fn toggle_profile_mod(
    app: tauri::AppHandle,
    app_id: String,
    mod_id: String,
    enabled: bool,
) -> Result<(), String> {
    caldera_backend::toggle_profile_mod(&app, app_id, mod_id, enabled).map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "camelCase")]
fn get_available_deployers(app: tauri::AppHandle) -> Result<Vec<DeployerOption>, String> {
    caldera_backend::get_available_deployers(&app).map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "camelCase")]
fn get_configured_deployer(app_id: String) -> Result<Option<String>, String> {
    caldera_backend::get_configured_deployer(app_id).map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "camelCase")]
fn set_game_deployer(
    app: tauri::AppHandle,
    app_id: String,
    deployer_id: String,
) -> Result<(), String> {
    caldera_backend::set_game_deployer(&app, app_id, deployer_id).map_err(|e| e.to_string())
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
    let result =
        caldera_backend::operations::uncompress::uncompress_with_progress(archive_path, |p| {
            emit(p, format!("Uncompressing... {}%", p));
        })
        .map_err(|e| e.to_string())?;
    emit(100, "Uncompress complete".to_string());
    Ok(result)
}

#[tauri::command(rename_all = "camelCase")]
fn deploy_listing(
    app: tauri::AppHandle,
    app_id: String,
    listing_id: String,
) -> Result<ModManifest, String> {
    caldera_backend::deploy_listing(&app, app_id, listing_id).map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "camelCase")]
async fn handle_nxm_link(app: tauri::AppHandle, url: String) -> Result<(), String> {
    caldera_backend::downloadmanagers::nexus_catcher::handle_nxm_link(app, url)
        .await
        .map_err(|e| e.to_string())
}

fn handle_nxm_arg(app: &tauri::AppHandle, arg: &str) {
    if !arg.starts_with("nxm://") {
        return;
    }

    let app = app.clone();
    let url = arg.to_string();
    tauri::async_runtime::spawn(async move {
        if let Err(err) =
            caldera_backend::downloadmanagers::nexus_catcher::handle_nxm_link(app.clone(), url)
                .await
        {
            let _ = app.emit(
                "caldera://session-log",
                serde_json::json!({
                    "message": err.to_string(),
                    "level": "error"
                }),
            );
        }
    });
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            for arg in argv {
                handle_nxm_arg(&app, &arg);
            }
        }))
        .setup(|app| {
            let app = app.handle().clone();
            for arg in std::env::args() {
                handle_nxm_arg(&app, &arg);
            }
            let deep_link_app = app.clone();
            app.deep_link().on_open_url(move |event| {
                let app = deep_link_app.clone();
                for url in event.urls() {
                    handle_nxm_arg(&app, url.as_str());
                }
            });
            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_deep_link::init())
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
            export_pack,
            import_pack,
            list_collections,
            fetch_nexus_collections,
            get_modlist_listings,
            open_downloads_folder,
            open_mod_deploy_folder,
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
            deploy_listing,
            handle_nxm_link
        ])
        .run(tauri::generate_context!())
        .expect("failed to run CALDERA Tauri app");
}

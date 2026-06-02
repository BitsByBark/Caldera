use std::fs;

use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Router,
};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[derive(Clone)]
struct DownloadState {
    app: AppHandle,
}

#[derive(Clone, Serialize)]
struct LogEvent {
    message: String,
    level: String,
}

#[derive(Deserialize)]
struct DownloadRequest {
    url: String,
    game_domain: String,
    mod_id: String,
    file_id: String,
}

pub async fn run_server(app: AppHandle) -> Result<(), String> {
    let state = DownloadState { app };
    let app = Router::new()
        .route("/download", post(handle_download))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:7337")
        .await
        .map_err(|e| format!("Failed binding download server: {}", e))?;

    axum::serve(listener, app)
        .await
        .map_err(|e| format!("Download server failed: {}", e))
}

async fn handle_download(
    State(state): State<DownloadState>,
    Json(payload): Json<DownloadRequest>,
) -> Response {
    match queue_download(state, payload).await {
        Ok(()) => StatusCode::OK.into_response(),
        Err((status, message)) => (status, message).into_response(),
    }
}

async fn queue_download(
    state: DownloadState,
    payload: DownloadRequest,
) -> Result<(), (StatusCode, String)> {
    validate_nexus_url(&payload.url)?;

    let api_key = nexus_api_key().map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let client = reqwest::Client::new();
    let base_url = "https://api.nexusmods.com/v1";

    let mod_details = fetch_nexus_json(
        &client,
        &api_key,
        &format!(
            "{}/games/{}/mods/{}.json",
            base_url, payload.game_domain, payload.mod_id
        ),
    )
    .await?;
    let files = fetch_nexus_json(
        &client,
        &api_key,
        &format!(
            "{}/games/{}/mods/{}/files.json",
            base_url, payload.game_domain, payload.mod_id
        ),
    )
    .await?;
    let download_links = fetch_optional_nexus_json(
        &client,
        &api_key,
        &format!(
            "{}/games/{}/mods/{}/files/{}/download_link.json",
            base_url, payload.game_domain, payload.mod_id, payload.file_id
        ),
    )
    .await?;
    let changelogs = fetch_nexus_json(
        &client,
        &api_key,
        &format!(
            "{}/games/{}/mods/{}/changelogs.json",
            base_url, payload.game_domain, payload.mod_id
        ),
    )
    .await?;

    let now = now_rfc3339();
    let metadata = json!({
        "fetched_at": now,
        "mod": mod_details,
        "files": files,
        "changelogs": changelogs,
        "download": {
            "url": payload.url.clone(),
            "file_id": payload.file_id.clone(),
            "queued_at": now,
            "links": download_links
        }
    });

    let cache_dir = crate::runtime::base_config_dir()
        .join("cache")
        .join(&payload.game_domain)
        .join(&payload.mod_id);
    fs::create_dir_all(&cache_dir).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed creating metadata cache {}: {}", cache_dir.display(), e),
        )
    })?;

    let meta_path = cache_dir.join("meta.json");
    let body = serde_json::to_string_pretty(&metadata).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed serializing Nexus metadata: {}", e),
        )
    })?;
    fs::write(&meta_path, body).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed writing metadata {}: {}", meta_path.display(), e),
        )
    })?;

    let mod_name = metadata
        .get("mod")
        .and_then(|m| m.get("name"))
        .and_then(Value::as_str)
        .unwrap_or("unknown mod");
    let message = format!("Download queued: {} ({})", mod_name, payload.url);
    println!("{}", message);
    let _ = state.app.emit(
        "caldera://session-log",
        LogEvent {
            message,
            level: "info".to_string(),
        },
    );

    Ok(())
}

fn validate_nexus_url(raw: &str) -> Result<(), (StatusCode, String)> {
    let url = Url::parse(raw).map_err(|_| (StatusCode::BAD_REQUEST, "invalid url".to_string()))?;
    let host = url
        .host_str()
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "invalid domain".to_string()))?;

    if host == "nexusmods.com" || host.ends_with(".nexusmods.com") {
        Ok(())
    } else {
        Err((StatusCode::BAD_REQUEST, "invalid domain".to_string()))
    }
}

fn nexus_api_key() -> Result<String, String> {
    let values = crate::runtime::get_settings_values()?;
    values
        .get("accounts")
        .and_then(|accounts| accounts.get("nexus_api_key"))
        .and_then(Value::as_str)
        .or_else(|| values.get("nexus_api_key").and_then(Value::as_str))
        .map(str::trim)
        .filter(|key| !key.is_empty())
        .map(str::to_string)
        .ok_or_else(|| "Missing Nexus API key at accounts.nexus_api_key".to_string())
}

async fn fetch_nexus_json(
    client: &reqwest::Client,
    api_key: &str,
    url: &str,
) -> Result<Value, (StatusCode, String)> {
    let response = client
        .get(url)
        .header("apikey", api_key)
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("Nexus request failed: {}", e)))?;

    let status = response.status();
    if !status.is_success() {
        return Err((
            StatusCode::BAD_GATEWAY,
            format!("Nexus returned {} for {}", status.as_u16(), url),
        ));
    }

    response
        .json::<Value>()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("Invalid Nexus JSON: {}", e)))
}

async fn fetch_optional_nexus_json(
    client: &reqwest::Client,
    api_key: &str,
    url: &str,
) -> Result<Value, (StatusCode, String)> {
    let response = client
        .get(url)
        .header("apikey", api_key)
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("Nexus request failed: {}", e)))?;

    let status = response.status();
    if !status.is_success() {
        return Ok(json!({
            "error": "nexus request failed",
            "status": status.as_u16(),
            "url": url
        }));
    }

    response
        .json::<Value>()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("Invalid Nexus JSON: {}", e)))
}

fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

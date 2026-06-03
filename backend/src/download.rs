use std::{fs, path::PathBuf};

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

#[derive(Clone, Deserialize)]
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
    let listener = tokio::net::TcpListener::bind("127.69.67.21:7337")
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

    let mod_root = mod_root_for_payload(&payload)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                format!(
                    "No configured game matches Nexus domain {}",
                    payload.game_domain
                ),
            )
        })?;
    fs::create_dir_all(mod_root.join("files")).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed creating mod storage {}: {}", mod_root.display(), e),
        )
    })?;

    let mod_name = metadata
        .get("mod")
        .and_then(|m| m.get("name"))
        .and_then(Value::as_str)
        .unwrap_or("unknown mod");
    let message = format!("Download queued: {} ({})", mod_name, payload.url);
    let listing_path = write_downloading_listing(&payload, &metadata, mod_name).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed writing download listing: {}", e),
        )
    })?;
    println!("{}", message);
    let _ = state.app.emit(
        "caldera://session-log",
        LogEvent {
            message,
            level: "info".to_string(),
        },
    );

    let app = state.app.clone();
    let client = client.clone();
    let payload_for_download = payload.clone();
    let metadata_for_download = metadata.clone();
    let mod_name = mod_name.to_string();
    tauri::async_runtime::spawn(async move {
        match download_archive(
            &client,
            &payload_for_download,
            &metadata_for_download,
            &mod_name,
        )
        .await
        {
            Ok(file_path) => {
                if let Some(listing_path) = listing_path.as_ref() {
                    if let Err(err) = mark_listing_downloaded(listing_path, &file_path) {
                        let _ = app.emit(
                            "caldera://session-log",
                            LogEvent {
                                message: format!("Failed updating download listing: {}", err),
                                level: "error".to_string(),
                            },
                        );
                    }
                }
                let complete_message =
                    format!("Download complete: {} ({})", mod_name, file_path.display());
                println!("{}", complete_message);
                let _ = app.emit(
                    "caldera://session-log",
                    LogEvent {
                        message: complete_message,
                        level: "success".to_string(),
                    },
                );
            }
            Err((_status, err)) => {
                if let Some(listing_path) = listing_path.as_ref() {
                    let _ = mark_listing_failed(listing_path, &err);
                }
                let failed_message = format!("Download failed: {} ({})", mod_name, err);
                eprintln!("{}", failed_message);
                let _ = app.emit(
                    "caldera://session-log",
                    LogEvent {
                        message: failed_message,
                        level: "error".to_string(),
                    },
                );
            }
        }
    });

    Ok(())
}

fn nexus_domain_for_name(name: &str) -> String {
    name.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect()
}

fn value_string(value: &Value, path: &[&str]) -> Option<String> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_str().map(str::to_string)
}

fn value_number(value: &Value, path: &[&str]) -> Option<f64> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_f64()
}

fn selected_file<'a>(metadata: &'a Value, file_id: &str) -> Option<&'a Value> {
    metadata
        .get("files")
        .and_then(|files| files.get("files"))
        .and_then(Value::as_array)
        .and_then(|files| {
            files.iter().find(|file| {
                file.get("file_id")
                    .and_then(Value::as_i64)
                    .map(|id| id.to_string() == file_id)
                    .unwrap_or(false)
            })
        })
}

fn matching_game_library_dir(game_domain: &str) -> Result<Option<PathBuf>, String> {
    let library_root = crate::runtime::base_config_dir().join("library");
    if !library_root.is_dir() {
        return Ok(None);
    }

    let entries = fs::read_dir(&library_root).map_err(|e| {
        format!(
            "Failed reading library dir {}: {}",
            library_root.display(),
            e
        )
    })?;
    for entry in entries.flatten() {
        let meta_path = entry.path().join("metadata").join("meta.json");
        if !meta_path.is_file() {
            continue;
        }
        let raw = match fs::read_to_string(&meta_path) {
            Ok(raw) => raw,
            Err(_) => continue,
        };
        let parsed = match serde_json::from_str::<Value>(&raw) {
            Ok(parsed) => parsed,
            Err(_) => continue,
        };
        let Some(app_id) = value_string(&parsed, &["app_id"]) else {
            continue;
        };
        let Some(name) = value_string(&parsed, &["name"]) else {
            continue;
        };
        if nexus_domain_for_name(&name) != game_domain {
            continue;
        }

        return Ok(Some(
            crate::runtime::base_config_dir()
                .join("library")
                .join(app_id),
        ));
    }

    Ok(None)
}

fn storage_mod_id(payload: &DownloadRequest) -> String {
    format!(
        "nexus-{}-{}-{}",
        payload.game_domain, payload.mod_id, payload.file_id
    )
}

fn mod_root_for_payload(payload: &DownloadRequest) -> Result<Option<PathBuf>, String> {
    Ok(matching_game_library_dir(&payload.game_domain)?
        .map(|root| root.join("mods").join(storage_mod_id(payload))))
}

fn files_dir_for_payload(payload: &DownloadRequest) -> Result<Option<PathBuf>, String> {
    Ok(mod_root_for_payload(payload)?.map(|root| root.join("files")))
}

fn write_downloading_listing(
    payload: &DownloadRequest,
    metadata: &Value,
    fallback_name: &str,
) -> Result<Option<PathBuf>, String> {
    let Some(mod_root) = mod_root_for_payload(payload)? else {
        return Ok(None);
    };

    fs::create_dir_all(mod_root.join("files")).map_err(|e| {
        format!(
            "Failed creating mod storage dir {}: {}",
            mod_root.display(),
            e
        )
    })?;

    let file = selected_file(metadata, &payload.file_id);
    let name = file
        .and_then(|f| value_string(f, &["file_name"]))
        .or_else(|| file.and_then(|f| value_string(f, &["name"])))
        .unwrap_or_else(|| fallback_name.to_string());
    let listing = crate::config::ModListing {
        mod_id: storage_mod_id(payload),
        name,
        status: "downloading".to_string(),
        source_path: Some(payload.url.clone()),
        deployable: false,
        deployer_reason: Some("Download still in progress".to_string()),
        added_at: value_string(metadata, &["download", "queued_at"]),
        progress: Some(0.0),
        speed: None,
        version: file
            .and_then(|f| value_string(f, &["version"]))
            .or_else(|| value_string(metadata, &["mod", "version"])),
        author: value_string(metadata, &["mod", "author"])
            .or_else(|| value_string(metadata, &["mod", "uploaded_by"])),
        description: file
            .and_then(|f| value_string(f, &["description"]))
            .or_else(|| value_string(metadata, &["mod", "description"])),
        summary: value_string(metadata, &["mod", "summary"]),
        source: Some("nexus".to_string()),
        source_url: Some(payload.url.clone()),
        nexus_mod_id: value_number(metadata, &["mod", "mod_id"]),
        nexus_file_id: payload.file_id.parse::<f64>().ok(),
        categories: Vec::new(),
        tags: Vec::new(),
        file_size: file.and_then(|f| value_number(f, &["size_in_bytes"])),
        file_count: None,
        file_types: Vec::new(),
        user_notes: None,
        favorite: None,
        files: Vec::new(),
    };
    let path = mod_root.join("meta.toml");
    let body = toml::to_string_pretty(&listing)
        .map_err(|e| format!("Failed serializing listing: {}", e))?;
    fs::write(&path, body)
        .map_err(|e| format!("Failed writing listing {}: {}", path.display(), e))?;
    Ok(Some(path))
}

fn download_link_url(links: &Value) -> Result<String, (StatusCode, String)> {
    if let Some(error) = links.get("error").and_then(Value::as_str) {
        let status = links
            .get("status")
            .and_then(Value::as_u64)
            .map(|s| s.to_string())
            .unwrap_or_else(|| "unknown".to_string());
        return Err((
            StatusCode::BAD_GATEWAY,
            format!("Nexus download link failed: {} ({})", error, status),
        ));
    }

    links
        .as_array()
        .and_then(|items| items.first())
        .and_then(|item| {
            item.get("URI")
                .or_else(|| item.get("uri"))
                .or_else(|| item.get("url"))
                .or_else(|| item.get("URL"))
        })
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| {
            (
                StatusCode::BAD_GATEWAY,
                "Nexus did not return a download URL".to_string(),
            )
        })
}

fn safe_file_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|ch| match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => ch,
        })
        .collect();
    let trimmed = cleaned.trim().trim_matches('.');
    if trimmed.is_empty() {
        "download.bin".to_string()
    } else {
        trimmed.to_string()
    }
}

fn selected_file_name(metadata: &Value, file_id: &str, fallback_name: &str) -> String {
    selected_file(metadata, file_id)
        .and_then(|file| value_string(file, &["file_name"]))
        .or_else(|| selected_file(metadata, file_id).and_then(|file| value_string(file, &["name"])))
        .unwrap_or_else(|| fallback_name.to_string())
}

async fn download_archive(
    client: &reqwest::Client,
    payload: &DownloadRequest,
    metadata: &Value,
    fallback_name: &str,
) -> Result<PathBuf, (StatusCode, String)> {
    let Some(download_dir) = files_dir_for_payload(payload).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed resolving download dir: {}", e),
        )
    })?
    else {
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "No configured game matches Nexus domain {}",
                payload.game_domain
            ),
        ));
    };

    fs::create_dir_all(&download_dir).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!(
                "Failed creating mod files dir {}: {}",
                download_dir.display(),
                e
            ),
        )
    })?;

    let link = metadata
        .get("download")
        .and_then(|download| download.get("links"))
        .ok_or_else(|| {
            (
                StatusCode::BAD_GATEWAY,
                "Missing Nexus download links".to_string(),
            )
        })
        .and_then(download_link_url)?;
    let file_name = safe_file_name(&selected_file_name(
        metadata,
        &payload.file_id,
        fallback_name,
    ));
    let final_path = download_dir.join(file_name);
    let part_path = final_path.with_extension(format!(
        "{}.part",
        final_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("download")
    ));

    let response = client.get(&link).send().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("Archive download request failed: {}", e),
        )
    })?;
    let status = response.status();
    if !status.is_success() {
        return Err((
            StatusCode::BAD_GATEWAY,
            format!("Archive download returned {}", status.as_u16()),
        ));
    }

    let bytes = response.bytes().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("Failed reading archive response: {}", e),
        )
    })?;
    fs::write(&part_path, &bytes).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed writing archive {}: {}", part_path.display(), e),
        )
    })?;
    fs::rename(&part_path, &final_path).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!(
                "Failed finalizing archive {} -> {}: {}",
                part_path.display(),
                final_path.display(),
                e
            ),
        )
    })?;

    Ok(final_path)
}

fn mark_listing_downloaded(listing_path: &PathBuf, file_path: &PathBuf) -> Result<(), String> {
    let raw = fs::read_to_string(listing_path)
        .map_err(|e| format!("Failed reading listing {}: {}", listing_path.display(), e))?;
    let mut listing = toml::from_str::<crate::config::ModListing>(&raw)
        .map_err(|e| format!("Invalid listing TOML {}: {}", listing_path.display(), e))?;
    listing.status = "downloaded".to_string();
    listing.source_path = Some(file_path.to_string_lossy().to_string());
    listing.progress = Some(1.0);
    listing.speed = Some("0 KB/S".to_string());
    listing.files = file_path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| vec![name.to_string()])
        .unwrap_or_default();
    let body = toml::to_string_pretty(&listing)
        .map_err(|e| format!("Failed serializing listing: {}", e))?;
    fs::write(listing_path, body)
        .map_err(|e| format!("Failed writing listing {}: {}", listing_path.display(), e))
}

fn mark_listing_failed(listing_path: &PathBuf, error: &str) -> Result<(), String> {
    let raw = fs::read_to_string(listing_path)
        .map_err(|e| format!("Failed reading listing {}: {}", listing_path.display(), e))?;
    let mut listing = toml::from_str::<crate::config::ModListing>(&raw)
        .map_err(|e| format!("Invalid listing TOML {}: {}", listing_path.display(), e))?;
    listing.status = "failed".to_string();
    listing.deployer_reason = Some(error.to_string());
    listing.progress = Some(0.0);
    let body = toml::to_string_pretty(&listing)
        .map_err(|e| format!("Failed serializing listing: {}", e))?;
    fs::write(listing_path, body)
        .map_err(|e| format!("Failed writing listing {}: {}", listing_path.display(), e))
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
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Nexus request failed: {}", e),
            )
        })?;

    let status = response.status();
    if !status.is_success() {
        return Err((
            StatusCode::BAD_GATEWAY,
            format!("Nexus returned {} for {}", status.as_u16(), url),
        ));
    }

    response.json::<Value>().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("Invalid Nexus JSON: {}", e),
        )
    })
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
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Nexus request failed: {}", e),
            )
        })?;

    let status = response.status();
    if !status.is_success() {
        return Ok(json!({
            "error": "nexus request failed",
            "status": status.as_u16(),
            "url": url
        }));
    }

    response.json::<Value>().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("Invalid Nexus JSON: {}", e),
        )
    })
}

fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

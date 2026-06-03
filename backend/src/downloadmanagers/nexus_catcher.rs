use std::{fs, io::Write, path::PathBuf};

use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::{AppError, WithPath};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NxmLink {
    pub game_domain: String,
    pub mod_id: u32,
    pub file_id: u32,
    pub key: Option<String>,
    pub expires: Option<u64>,
    pub user_id: Option<u32>,
}

#[derive(Clone, Serialize)]
struct LogEvent {
    message: String,
    level: String,
}

#[derive(Clone, Serialize)]
struct DownloadProgressEvent {
    key: String,
    game_domain: String,
    mod_id: u32,
    file_id: u32,
    name: String,
    progress: u8,
    message: String,
}

pub fn parse_nxm(raw: &str) -> Result<NxmLink, AppError> {
    let url =
        Url::parse(raw).map_err(|e| AppError::other(format!("Invalid NXM link: {}", e)))?;
    if url.scheme() != "nxm" {
        return Err(AppError::other("Invalid NXM link: scheme must be nxm"));
    }

    let game_domain = url
        .host_str()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| AppError::other("Invalid NXM link: missing game domain"))?
        .to_string();
    let segments = url
        .path_segments()
        .ok_or_else(|| AppError::other("Invalid NXM link: missing path"))?
        .collect::<Vec<_>>();
    if segments.len() != 4 || segments[0] != "mods" || segments[2] != "files" {
        return Err(AppError::other(
            "Invalid NXM link: expected /mods/{mod_id}/files/{file_id}",
        ));
    }

    let mod_id = segments[1]
        .parse::<u32>()
        .map_err(|_| AppError::other("Invalid NXM link: mod_id must be numeric"))?;
    let file_id = segments[3]
        .parse::<u32>()
        .map_err(|_| AppError::other("Invalid NXM link: file_id must be numeric"))?;

    let mut key = None;
    let mut expires = None;
    let mut user_id = None;
    for (k, v) in url.query_pairs() {
        match k.as_ref() {
            "key" => key = Some(v.to_string()),
            "expires" => {
                expires = Some(
                    v.parse::<u64>()
                        .map_err(|_| AppError::other("Invalid NXM link: expires must be numeric"))?,
                )
            }
            "user_id" => {
                user_id = Some(
                    v.parse::<u32>()
                        .map_err(|_| AppError::other("Invalid NXM link: user_id must be numeric"))?,
                )
            }
            _ => {}
        }
    }

    Ok(NxmLink {
        game_domain,
        mod_id,
        file_id,
        key,
        expires,
        user_id,
    })
}

pub async fn handle_nxm_link(app: AppHandle, url: String) -> Result<(), AppError> {
    let link = match parse_nxm(&url) {
        Ok(link) => link,
        Err(err) => {
            emit_log(&app, "Invalid NXM link", "error");
            return Err(err);
        }
    };

    emit_log(
        &app,
        &format!(
            "Received NXM link: {}/{}/{}",
            link.game_domain, link.mod_id, link.file_id
        ),
        "info",
    );

    let api_key = match nexus_api_key() {
        Ok(api_key) => api_key,
        Err(err) => {
            emit_log(&app, "No Nexus API key set - add one in settings", "error");
            return Err(err);
        }
    };

    let client = reqwest::Client::new();
    let base_url = "https://api.nexusmods.com/v1";
    let download_links = resolve_download_links(&client, &api_key, &link).await;

    let (links, links_error, resolved_uri) = match download_links {
        Ok(links) => {
            emit_log(&app, "Resolved download URL", "success");
            let resolved_uri = first_download_uri(&links)
                .map(Value::String)
                .unwrap_or(Value::Null);
            (links, Value::Null, resolved_uri)
        }
        Err((status, err)) => {
            emit_log(
                &app,
                &format!("Could not resolve download link: {}", status),
                "warning",
            );
            (
                Value::Null,
                json!({ "status": status, "error": err }),
                Value::Null,
            )
        }
    };

    let mod_details = fetch_nexus_json(
        &client,
        &api_key,
        &format!(
            "{}/games/{}/mods/{}.json",
            base_url, link.game_domain, link.mod_id
        ),
    )
    .await?;
    let files = fetch_nexus_json(
        &client,
        &api_key,
        &format!(
            "{}/games/{}/mods/{}/files.json",
            base_url, link.game_domain, link.mod_id
        ),
    )
    .await?;
    let changelogs = fetch_nexus_json(
        &client,
        &api_key,
        &format!(
            "{}/games/{}/mods/{}/changelogs.json",
            base_url, link.game_domain, link.mod_id
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
            "url": url,
            "file_id": link.file_id,
            "queued_at": now,
            "links": links,
            "links_error": links_error,
            "resolved_uri": resolved_uri,
            "nxm": {
                "key": link.key,
                "expires": link.expires,
                "user_id": link.user_id
            }
        }
    });
    write_metadata(&link, &metadata)?;
    append_queue_item(&link, &metadata)?;

    let mod_name = metadata
        .get("mod")
        .and_then(|m| m.get("name"))
        .and_then(Value::as_str)
        .unwrap_or("unknown mod");
    let listing_path = write_listing(&link, &metadata, mod_name, 0.0, "downloading", None)?;
    emit_log(&app, &format!("Download queued: {}", mod_name), "success");
    emit_download_event(&app, &link, mod_name);
    emit_download_progress(&app, &link, mod_name, 0, "Download started");

    let app_for_download = app.clone();
    let client_for_download = client.clone();
    let link_for_download = link.clone();
    let metadata_for_download = metadata.clone();
    let mod_name = mod_name.to_string();
    tauri::async_runtime::spawn(async move {
        match download_archive(
            &app_for_download,
            &client_for_download,
            &link_for_download,
            &metadata_for_download,
            &listing_path,
            &mod_name,
        )
        .await
        {
            Ok(file_path) => {
                if let Err(err) = update_listing_downloaded(&listing_path, &file_path) {
                    emit_log(
                        &app_for_download,
                        &format!("Failed updating listing: {}", err),
                        "error",
                    );
                }
                emit_download_progress(
                    &app_for_download,
                    &link_for_download,
                    &mod_name,
                    100,
                    "Download complete",
                );
                emit_log(
                    &app_for_download,
                    &format!("Download complete: {}", mod_name),
                    "success",
                );
            }
            Err(err) => {
                let _ = update_listing_failed(&listing_path, &err.to_string());
                emit_log(
                    &app_for_download,
                    &format!("Download failed: {} ({})", mod_name, err),
                    "error",
                );
            }
        }
    });

    Ok(())
}

async fn resolve_download_links(
    client: &reqwest::Client,
    api_key: &str,
    link: &NxmLink,
) -> Result<Value, (u16, String)> {
    let mut url = Url::parse(&format!(
        "https://api.nexusmods.com/v1/games/{}/mods/{}/files/{}/download_link.json",
        link.game_domain, link.mod_id, link.file_id
    ))
    .map_err(|e| (500, format!("Invalid Nexus URL: {}", e)))?;
    if let (Some(key), Some(expires)) = (link.key.as_ref(), link.expires) {
        url.query_pairs_mut()
            .append_pair("key", key)
            .append_pair("expires", &expires.to_string());
    }

    let response = client
        .get(url)
        .header("apikey", api_key)
        .send()
        .await
        .map_err(|e| (502, format!("Nexus request failed: {}", e)))?;
    let status = response.status();
    if !status.is_success() {
        return Err((status.as_u16(), "nexus request failed".to_string()));
    }

    response
        .json::<Value>()
        .await
        .map_err(|e| (502, format!("Invalid Nexus JSON: {}", e)))
}

async fn fetch_nexus_json(
    client: &reqwest::Client,
    api_key: &str,
    url: &str,
) -> Result<Value, AppError> {
    let response = client
        .get(url)
        .header("apikey", api_key)
        .send()
        .await
        .map_err(|e| AppError::other(format!("Nexus request failed: {}", e)))?;
    let status = response.status();
    if !status.is_success() {
        return Err(AppError::other(format!(
            "Nexus returned {} for {}",
            status.as_u16(),
            url
        )));
    }

    response
        .json::<Value>()
        .await
        .map_err(|e| AppError::other(format!("Invalid Nexus JSON: {}", e)))
}

fn write_metadata(link: &NxmLink, _metadata: &Value) -> Result<(), AppError> {
    let mod_dir = mod_root_for_link(link)?;
    let files_dir = mod_dir.join("files");
    fs::create_dir_all(&files_dir).with_path(&files_dir)
}

fn append_queue_item(link: &NxmLink, metadata: &Value) -> Result<(), AppError> {
    let queue_path = crate::filehandler::runtime::base_config_dir().join("download_queue.json");
    let mut queue = if queue_path.exists() {
        let raw = fs::read_to_string(&queue_path).with_path(&queue_path)?;
        serde_json::from_str::<Vec<Value>>(&raw).unwrap_or_default()
    } else {
        Vec::new()
    };

    queue.push(json!({
        "url": metadata.get("download").and_then(|d| d.get("resolved_uri")).cloned().unwrap_or(Value::Null),
        "game_domain": link.game_domain,
        "mod_id": link.mod_id,
        "file_id": link.file_id,
        "queued_at": metadata.get("download").and_then(|d| d.get("queued_at")).cloned().unwrap_or(Value::Null),
    }));

    if let Some(parent) = queue_path.parent() {
        fs::create_dir_all(parent).with_path(parent)?;
    }
    let body = serde_json::to_string_pretty(&queue).map_err(AppError::Json)?;
    fs::write(&queue_path, body).with_path(&queue_path)
}

fn first_download_uri(links: &Value) -> Option<String> {
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

fn selected_file<'a>(metadata: &'a Value, file_id: u32) -> Option<&'a Value> {
    metadata
        .get("files")
        .and_then(|files| files.get("files"))
        .and_then(Value::as_array)
        .and_then(|files| {
            files.iter().find(|file| {
                file.get("file_id")
                    .and_then(Value::as_u64)
                    .map(|id| id == file_id as u64)
                    .unwrap_or(false)
            })
        })
}

fn selected_file_name(metadata: &Value, file_id: u32, fallback_name: &str) -> String {
    selected_file(metadata, file_id)
        .and_then(|file| value_string(file, &["file_name"]))
        .or_else(|| selected_file(metadata, file_id).and_then(|file| value_string(file, &["name"])))
        .unwrap_or_else(|| fallback_name.to_string())
}

fn safe_file_name(name: &str) -> String {
    let cleaned = name
        .chars()
        .map(|ch| match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => ch,
        })
        .collect::<String>();
    let trimmed = cleaned.trim().trim_matches('.');
    if trimmed.is_empty() {
        "download.bin".to_string()
    } else {
        trimmed.to_string()
    }
}

fn library_root_for_domain(game_domain: &str) -> Result<PathBuf, AppError> {
    let library_root = crate::filehandler::runtime_reader::library_dir();
    let entries = fs::read_dir(&library_root).with_path(&library_root)?;
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
        if nexus_domain_for_name(&name) == game_domain {
            return Ok(crate::filehandler::runtime_reader::game_dir(&app_id));
        }
    }

    Err(AppError::other(format!(
        "No configured game matches Nexus domain {}",
        game_domain
    )))
}

fn storage_mod_id(link: &NxmLink) -> String {
    format!(
        "nexus-{}-{}-{}",
        link.game_domain, link.mod_id, link.file_id
    )
}

fn mod_root_for_link(link: &NxmLink) -> Result<PathBuf, AppError> {
    Ok(library_root_for_domain(&link.game_domain)?
        .join("mods")
        .join(storage_mod_id(link)))
}

fn download_dir_for_link(link: &NxmLink) -> Result<PathBuf, AppError> {
    Ok(mod_root_for_link(link)?.join("files"))
}

fn listing_path(link: &NxmLink) -> Result<PathBuf, AppError> {
    Ok(mod_root_for_link(link)?.join("meta.toml"))
}

fn write_listing(
    link: &NxmLink,
    metadata: &Value,
    fallback_name: &str,
    progress: f32,
    status: &str,
    source_path: Option<String>,
) -> Result<PathBuf, AppError> {
    let path = listing_path(link)?;
    let parent = path
        .parent()
        .ok_or_else(|| AppError::other(format!("Invalid listing path {}", path.display())))?;
    fs::create_dir_all(parent).with_path(parent)?;
    let file = selected_file(metadata, link.file_id);
    let listing = crate::config::ModListing {
        mod_id: storage_mod_id(link),
        name: selected_file_name(metadata, link.file_id, fallback_name),
        status: status.to_string(),
        source_path,
        deployable: false,
        deployer_reason: None,
        added_at: value_string(metadata, &["download", "queued_at"]),
        progress: Some(progress),
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
        source_url: value_string(metadata, &["download", "url"]),
        nexus_mod_id: Some(link.mod_id as f64),
        nexus_file_id: Some(link.file_id as f64),
        categories: Vec::new(),
        tags: Vec::new(),
        file_size: file.and_then(|f| value_number(f, &["size_in_bytes"])),
        file_count: None,
        file_types: Vec::new(),
        user_notes: None,
        favorite: None,
        files: Vec::new(),
    };
    write_listing_value(&path, &listing)?;
    Ok(path)
}

fn read_listing(path: &PathBuf) -> Result<crate::config::ModListing, AppError> {
    let raw = fs::read_to_string(path).with_path(path)?;
    toml::from_str::<crate::config::ModListing>(&raw).map_err(AppError::TomlParse)
}

fn write_listing_value(
    path: &PathBuf,
    listing: &crate::config::ModListing,
) -> Result<(), AppError> {
    let body = toml::to_string_pretty(listing).map_err(AppError::TomlSerialize)?;
    fs::write(path, body).with_path(path)
}

fn update_listing_progress(path: &PathBuf, progress: f32) -> Result<(), AppError> {
    let mut listing = read_listing(path)?;
    listing.status = "downloading".to_string();
    listing.progress = Some(progress.clamp(0.0, 1.0));
    write_listing_value(path, &listing)
}

fn update_listing_downloaded(path: &PathBuf, file_path: &PathBuf) -> Result<(), AppError> {
    let mut listing = read_listing(path)?;
    listing.status = "downloaded".to_string();
    listing.source_path = Some(file_path.to_string_lossy().to_string());
    listing.progress = Some(1.0);
    listing.speed = Some("0 KB/S".to_string());
    listing.files = file_path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| vec![name.to_string()])
        .unwrap_or_default();
    write_listing_value(path, &listing)
}

fn update_listing_failed(path: &PathBuf, error: &str) -> Result<(), AppError> {
    let mut listing = read_listing(path)?;
    listing.status = "failed".to_string();
    listing.deployer_reason = Some(error.to_string());
    write_listing_value(path, &listing)
}

async fn download_archive(
    app: &AppHandle,
    client: &reqwest::Client,
    link: &NxmLink,
    metadata: &Value,
    listing_path: &PathBuf,
    mod_name: &str,
) -> Result<PathBuf, AppError> {
    let download_url = value_string(metadata, &["download", "resolved_uri"])
        .ok_or_else(|| AppError::other("No resolved Nexus download URL"))?;
    let download_dir = download_dir_for_link(link)?;
    fs::create_dir_all(&download_dir).with_path(&download_dir)?;

    let file_name = safe_file_name(&selected_file_name(metadata, link.file_id, mod_name));
    let final_path = download_dir.join(file_name);
    let part_path = final_path.with_extension(format!(
        "{}.part",
        final_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("download")
    ));

    let mut response = client
        .get(download_url)
        .send()
        .await
        .map_err(|e| AppError::other(format!("Archive download request failed: {}", e)))?;
    let status = response.status();
    if !status.is_success() {
        return Err(AppError::other(format!(
            "Archive download returned {}",
            status.as_u16()
        )));
    }

    let total = response.content_length().unwrap_or(0);
    let mut downloaded = 0_u64;
    let mut last_percent = 0_u8;
    let mut out = fs::File::create(&part_path).with_path(&part_path)?;

    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| AppError::other(format!("Failed reading archive response: {}", e)))?
    {
        out.write_all(&chunk).with_path(&part_path)?;
        downloaded += chunk.len() as u64;
        let percent = if total > 0 {
            ((downloaded.saturating_mul(100) / total).min(100)) as u8
        } else {
            0
        };
        if percent != last_percent && (percent == 100 || percent >= last_percent.saturating_add(5))
        {
            last_percent = percent;
            let progress = if total > 0 {
                percent as f32 / 100.0
            } else {
                0.0
            };
            let _ = update_listing_progress(listing_path, progress);
            emit_download_progress(app, link, mod_name, percent, "Downloading archive");
        }
    }

    fs::rename(&part_path, &final_path).with_path(&final_path)?;
    Ok(final_path)
}

fn nexus_api_key() -> Result<String, AppError> {
    let values = crate::filehandler::runtime::get_settings_values()?;
    values
        .get("accounts")
        .and_then(|accounts| accounts.get("nexus_api_key"))
        .and_then(Value::as_str)
        .or_else(|| values.get("nexus_api_key").and_then(Value::as_str))
        .map(str::trim)
        .filter(|key| !key.is_empty())
        .map(str::to_string)
        .ok_or_else(|| AppError::other("No Nexus API key set - add one in settings"))
}

fn emit_log(app: &AppHandle, message: &str, level: &str) {
    let _ = app.emit(
        "caldera://session-log",
        LogEvent {
            message: message.to_string(),
            level: level.to_string(),
        },
    );
}

fn emit_download_event(app: &AppHandle, link: &NxmLink, mod_name: &str) {
    let _ = app.emit(
        "caldera://download-queued",
        json!({
            "game_domain": link.game_domain,
            "mod_id": link.mod_id,
            "file_id": link.file_id,
            "name": mod_name,
        }),
    );
}

fn emit_download_progress(
    app: &AppHandle,
    link: &NxmLink,
    mod_name: &str,
    progress: u8,
    message: &str,
) {
    let _ = app.emit(
        "caldera://download-progress",
        DownloadProgressEvent {
            key: format!(
                "download:{}:{}:{}",
                link.game_domain, link.mod_id, link.file_id
            ),
            game_domain: link.game_domain.clone(),
            mod_id: link.mod_id,
            file_id: link.file_id,
            name: mod_name.to_string(),
            progress,
            message: format!("{}: {}", message, mod_name),
        },
    );
}

fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

#[cfg(test)]
mod tests {
    use super::parse_nxm;

    #[test]
    fn parses_nxm_link() {
        let parsed = parse_nxm(
            "nxm://cyberpunk2077/mods/1234/files/5678?key=abc123&expires=1730000000&user_id=99999",
        )
        .unwrap();
        assert_eq!(parsed.game_domain, "cyberpunk2077");
        assert_eq!(parsed.mod_id, 1234);
        assert_eq!(parsed.file_id, 5678);
        assert_eq!(parsed.key.as_deref(), Some("abc123"));
        assert_eq!(parsed.expires, Some(1730000000));
        assert_eq!(parsed.user_id, Some(99999));
    }
}

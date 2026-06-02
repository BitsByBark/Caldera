use std::fs;

use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

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

pub fn parse_nxm(raw: &str) -> Result<NxmLink, String> {
    let url = Url::parse(raw).map_err(|e| format!("Invalid NXM link: {}", e))?;
    if url.scheme() != "nxm" {
        return Err("Invalid NXM link: scheme must be nxm".to_string());
    }

    let game_domain = url
        .host_str()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| "Invalid NXM link: missing game domain".to_string())?
        .to_string();
    let segments = url
        .path_segments()
        .ok_or_else(|| "Invalid NXM link: missing path".to_string())?
        .collect::<Vec<_>>();
    if segments.len() != 4 || segments[0] != "mods" || segments[2] != "files" {
        return Err("Invalid NXM link: expected /mods/{mod_id}/files/{file_id}".to_string());
    }

    let mod_id = segments[1]
        .parse::<u32>()
        .map_err(|_| "Invalid NXM link: mod_id must be numeric".to_string())?;
    let file_id = segments[3]
        .parse::<u32>()
        .map_err(|_| "Invalid NXM link: file_id must be numeric".to_string())?;

    let mut key = None;
    let mut expires = None;
    let mut user_id = None;
    for (k, v) in url.query_pairs() {
        match k.as_ref() {
            "key" => key = Some(v.to_string()),
            "expires" => {
                expires = Some(
                    v.parse::<u64>()
                        .map_err(|_| "Invalid NXM link: expires must be numeric".to_string())?,
                )
            }
            "user_id" => {
                user_id = Some(
                    v.parse::<u32>()
                        .map_err(|_| "Invalid NXM link: user_id must be numeric".to_string())?,
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

pub async fn handle_nxm_link(app: AppHandle, url: String) -> Result<(), String> {
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
    emit_log(&app, &format!("Download queued: {}", mod_name), "success");
    emit_download_event(&app, &link, mod_name);
    emit_log(&app, "NXM archive download stubbed for now", "info");

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
) -> Result<Value, String> {
    let response = client
        .get(url)
        .header("apikey", api_key)
        .send()
        .await
        .map_err(|e| format!("Nexus request failed: {}", e))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("Nexus returned {} for {}", status.as_u16(), url));
    }

    response
        .json::<Value>()
        .await
        .map_err(|e| format!("Invalid Nexus JSON: {}", e))
}

fn write_metadata(link: &NxmLink, metadata: &Value) -> Result<(), String> {
    let cache_dir = crate::runtime::base_config_dir()
        .join("cache")
        .join(&link.game_domain)
        .join(link.mod_id.to_string());
    fs::create_dir_all(&cache_dir).map_err(|e| {
        format!(
            "Failed creating metadata cache {}: {}",
            cache_dir.display(),
            e
        )
    })?;
    let meta_path = cache_dir.join("meta.json");
    let body = serde_json::to_string_pretty(metadata)
        .map_err(|e| format!("Failed serializing Nexus metadata: {}", e))?;
    fs::write(&meta_path, body)
        .map_err(|e| format!("Failed writing metadata {}: {}", meta_path.display(), e))
}

fn append_queue_item(link: &NxmLink, metadata: &Value) -> Result<(), String> {
    let queue_path = crate::runtime::base_config_dir().join("download_queue.json");
    let mut queue = if queue_path.exists() {
        let raw = fs::read_to_string(&queue_path)
            .map_err(|e| format!("Failed reading queue {}: {}", queue_path.display(), e))?;
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
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed creating queue dir {}: {}", parent.display(), e))?;
    }
    let body = serde_json::to_string_pretty(&queue)
        .map_err(|e| format!("Failed serializing queue: {}", e))?;
    fs::write(&queue_path, body)
        .map_err(|e| format!("Failed writing queue {}: {}", queue_path.display(), e))
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
        .ok_or_else(|| "No Nexus API key set - add one in settings".to_string())
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

use std::fs;
use std::path::{Path, PathBuf};

use crate::{AppError, WithPath};

use super::{get_deployer_override_path, read_game_meta, DeployLogger, DeployerConfig};

pub fn resolve_unreal_mod_path(
    app_id: &str,
    cfg: &DeployerConfig,
    logger: &impl DeployLogger,
) -> Result<PathBuf, AppError> {
    logger.info("Resolving Unreal Engine mod path...");

    if let Some(override_path) = get_deployer_override_path(app_id)? {
        logger.success(&format!("Found content path: {}", override_path.display()));
        return Ok(override_path);
    }

    let meta = read_game_meta(app_id)?;
    if meta.install_path.trim().is_empty() {
        logger.warning("Could not locate Content/Paks — set path manually in setup");
        return Err(AppError::other("Game install path missing in cache metadata"));
    }

    let install_root = PathBuf::from(meta.install_path);
    if !install_root.exists() {
        logger.warning("Could not locate Content/Paks — set path manually in setup");
        return Err(AppError::other(format!(
            "Install root does not exist: {}",
            install_root.display()
        )));
    }

    let hint = Path::new(&cfg.content_path_hint);
    let mut found_content: Option<PathBuf> = None;

    let entries = fs::read_dir(&install_root).with_path(&install_root)?;

    for entry in entries.flatten() {
        let p = entry.path();
        if !p.is_dir() {
            continue;
        }
        let candidate = p.join(hint);
        if candidate.is_dir() {
            found_content = Some(candidate);
            break;
        }
    }

    let Some(content_path) = found_content else {
        logger.warning("Could not locate Content/Paks — set path manually in setup");
        return Err(AppError::other(
            "Could not locate Content/Paks. Set deployer_mod_path manually in setup.",
        ));
    };

    logger.success(&format!("Found content path: {}", content_path.display()));
    Ok(content_path.join(&cfg.mod_subfolder))
}

use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use zip::ZipArchive;

fn is_supported_archive(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("zip"))
        .unwrap_or(false)
}

fn sanitize_zip_path(name: &str) -> PathBuf {
    let mut out = PathBuf::new();
    for c in Path::new(name).components() {
        match c {
            Component::Normal(seg) => out.push(seg),
            Component::CurDir => {}
            _ => {}
        }
    }
    out
}

pub fn uncompress_with_progress<F>(
    archive_path: String,
    mut on_progress: F,
) -> Result<Vec<String>, String>
where
    F: FnMut(u8),
{
    let src = PathBuf::from(archive_path);
    if !src.exists() || !src.is_file() {
        return Err(format!("Archive not found: {}", src.display()));
    }
    if !is_supported_archive(&src) {
        return Err("Unsupported archive format. Only .zip is supported right now.".to_string());
    }

    let target_dir = src
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "Could not resolve archive parent directory".to_string())?;

    let file = fs::File::open(&src)
        .map_err(|e| format!("Failed opening archive {}: {}", src.display(), e))?;
    let mut zip = ZipArchive::new(file)
        .map_err(|e| format!("Invalid zip archive {}: {}", src.display(), e))?;

    let mut extracted = Vec::new();
    let total = zip.len().max(1);
    on_progress(0);
    for i in 0..zip.len() {
        let mut entry = zip
            .by_index(i)
            .map_err(|e| format!("Failed reading zip entry #{}: {}", i, e))?;

        let rel = sanitize_zip_path(entry.name());
        if rel.as_os_str().is_empty() {
            continue;
        }
        let out_path = target_dir.join(&rel);

        if entry.is_dir() {
            fs::create_dir_all(&out_path)
                .map_err(|e| format!("Failed creating directory {}: {}", out_path.display(), e))?;
            continue;
        }

        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed creating directory {}: {}", parent.display(), e))?;
        }

        let mut out_file = fs::File::create(&out_path)
            .map_err(|e| format!("Failed creating file {}: {}", out_path.display(), e))?;
        io::copy(&mut entry, &mut out_file)
            .map_err(|e| format!("Failed writing file {}: {}", out_path.display(), e))?;

        extracted.push(out_path.to_string_lossy().to_string());
        let pct = (((i + 1) as f32 / total as f32) * 100.0).round() as u8;
        on_progress(pct.min(100));
    }

    Ok(extracted)
}

pub fn uncompress(archive_path: String) -> Result<Vec<String>, String> {
    uncompress_with_progress(archive_path, |_| {})
}

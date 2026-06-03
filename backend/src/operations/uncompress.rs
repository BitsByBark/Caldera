use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use zip::ZipArchive;

use crate::{AppError, WithPath};

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

#[derive(Debug)]
struct ExtractedEntry {
    path: PathBuf,
    is_dir: bool,
    expected_size: u64,
}

fn verify_extraction(entries: &[ExtractedEntry]) -> Result<(), AppError> {
    if entries.is_empty() {
        return Err(AppError::other("Archive contained no extractable entries"));
    }

    let mut file_count = 0usize;
    let mut dir_count = 0usize;

    for entry in entries {
        if entry.is_dir {
            if !entry.path.is_dir() {
                return Err(AppError::other(format!(
                    "Extracted directory missing: {}",
                    entry.path.display()
                )));
            }
            dir_count += 1;
            continue;
        }

        file_count += 1;
        let meta = fs::metadata(&entry.path).with_path(&entry.path)?;
        if !meta.is_file() {
            return Err(AppError::other(format!(
                "Extracted path is not a file: {}",
                entry.path.display()
            )));
        }
        let actual = meta.len();
        if actual != entry.expected_size {
            return Err(AppError::other(format!(
                "Size mismatch for {}: expected {} bytes, got {}",
                entry.path.display(),
                entry.expected_size,
                actual
            )));
        }
    }

    if file_count == 0 && dir_count == 0 {
        return Err(AppError::other(
            "Uncompress verification failed: no files or directories were written",
        ));
    }

    Ok(())
}

pub fn uncompress_with_progress<F>(
    archive_path: String,
    mut on_progress: F,
) -> Result<Vec<String>, AppError>
where
    F: FnMut(u8),
{
    let src = PathBuf::from(&archive_path);
    if !src.exists() || !src.is_file() {
        return Err(AppError::other(format!(
            "Archive not found: {}",
            src.display()
        )));
    }
    if !is_supported_archive(&src) {
        return Err(AppError::other(
            "Unsupported archive format. Only .zip is supported right now.",
        ));
    }

    let target_dir = src
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| AppError::other("Could not resolve archive parent directory"))?;

    let file = fs::File::open(&src).with_path(&src)?;
    let mut zip = ZipArchive::new(file)
        .map_err(|e| AppError::other(format!("Invalid zip archive {}: {}", src.display(), e)))?;

    let mut extracted_entries = Vec::new();
    let total = zip.len().max(1);
    on_progress(0);
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| {
            AppError::other(format!("Failed reading zip entry #{}: {}", i, e))
        })?;

        let rel = sanitize_zip_path(entry.name());
        if rel.as_os_str().is_empty() {
            continue;
        }
        let out_path = target_dir.join(&rel);
        let is_dir = entry.is_dir();

        if is_dir {
            fs::create_dir_all(&out_path).with_path(&out_path)?;
            extracted_entries.push(ExtractedEntry {
                path: out_path,
                is_dir: true,
                expected_size: 0,
            });
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent).with_path(parent)?;
            }

            let expected_size = entry.size();
            let mut out_file = fs::File::create(&out_path).with_path(&out_path)?;
            io::copy(&mut entry, &mut out_file).with_path(&out_path)?;

            extracted_entries.push(ExtractedEntry {
                path: out_path,
                is_dir: false,
                expected_size,
            });
        }

        let pct = (((i + 1) as f32 / total as f32) * 100.0).round() as u8;
        on_progress(pct.min(99));
    }

    verify_extraction(&extracted_entries)?;

    let extracted_files: Vec<PathBuf> = extracted_entries
        .iter()
        .filter(|e| !e.is_dir)
        .map(|e| e.path.clone())
        .collect();

    crate::config::update_listing_after_uncompress(&src, &target_dir, &extracted_files)?;

    fs::remove_file(&src).with_path(&src)?;

    on_progress(100);

    Ok(extracted_entries
        .into_iter()
        .map(|e| e.path.to_string_lossy().to_string())
        .collect())
}

pub fn uncompress(archive_path: String) -> Result<Vec<String>, AppError> {
    uncompress_with_progress(archive_path, |_| {})
}

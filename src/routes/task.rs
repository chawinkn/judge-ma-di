use axum::{
    extract::{Multipart, Path},
    http::header,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::io::{Cursor, Read};

use crate::error::{json_response, AppError, ResponseCode};
use crate::judge::config::get_task_config;

const MAX_ZIP_TOTAL_SIZE: u64 = 256 * 1024 * 1024; // 256 MB
const MAX_ZIP_FILES: usize = 1000;

fn safe_extract_zip(data: &[u8], target: &std::path::Path) -> Result<(), AppError> {
    let extract = || -> Result<(), AppError> {
        let mut archive = zip::ZipArchive::new(Cursor::new(data))
            .map_err(|e| AppError::BadRequest(format!("Invalid zip archive: {e}")))?;

        if archive.len() > MAX_ZIP_FILES {
            return Err(AppError::BadRequest(format!(
                "Zip contains too many files (max: {MAX_ZIP_FILES})"
            )));
        }

        let mut total_uncompressed_bytes: u64 = 0;

        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|e| AppError::BadRequest(format!("Corrupt zip entry: {e}")))?;

            let enclosed_path = file.enclosed_name().ok_or_else(|| {
                AppError::BadRequest("Zip entry contains illegal path traversal".to_string())
            })?;

            let out_path = target.join(enclosed_path);

            if file.is_dir() {
                std::fs::create_dir_all(&out_path)?;
            } else {
                if let Some(parent) = out_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut outfile = std::fs::File::create(&out_path)?;
                let mut limited = (&mut file)
                    .take(MAX_ZIP_TOTAL_SIZE.saturating_sub(total_uncompressed_bytes) + 1);
                let written = std::io::copy(&mut limited, &mut outfile)?;
                total_uncompressed_bytes += written;

                if total_uncompressed_bytes > MAX_ZIP_TOTAL_SIZE {
                    return Err(AppError::BadRequest(format!(
                        "Zip exceeds maximum uncompressed size of {} MB",
                        MAX_ZIP_TOTAL_SIZE / (1024 * 1024)
                    )));
                }
            }
        }

        Ok(())
    };

    let res = extract();
    if res.is_err() {
        let _ = std::fs::remove_dir_all(target);
    }
    res
}

fn validate_task_id(task_id: &str) -> Result<(), AppError> {
    if task_id.is_empty()
        || !task_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(AppError::BadRequest("invalid task_id".to_string()));
    }
    Ok(())
}

pub async fn get_task_testcases(
    Path(task_id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    validate_task_id(&task_id)?;
    let contents = tokio::fs::read(format!("tasks/{task_id}/testcases.zip"))
        .await
        .map_err(|_| AppError::NotFound(format!("Testcases for task '{task_id}' not found")))?;

    Ok((
        [
            (header::CONTENT_TYPE, "application/zip"),
            (
                header::CONTENT_DISPOSITION,
                "inline; filename=\"testcases.zip\"",
            ),
        ],
        contents,
    ))
}

// TODO: Auth
pub async fn upload_task(
    Path(task_id): Path<String>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    validate_task_id(&task_id)?;
    let dir = format!("tasks/{task_id}");
    tokio::fs::create_dir_all(&dir).await?;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?
    {
        let raw_name = field
            .file_name()
            .ok_or_else(|| AppError::BadRequest("missing filename".to_string()))?;
        let safe_name = std::path::Path::new(raw_name)
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| AppError::BadRequest("invalid filename".to_string()))?
            .to_string();
        let data = field
            .bytes()
            .await
            .map_err(|e| AppError::BadRequest(e.to_string()))?;

        tokio::fs::write(format!("{dir}/{safe_name}"), &data).await?;

        if safe_name.ends_with(".zip") {
            let target = format!("{dir}/testcases");
            let _ = tokio::fs::remove_dir_all(&target).await;
            safe_extract_zip(&data, std::path::Path::new(&target))?;
        }
    }

    Ok(json_response(ResponseCode::Ok))
}

// TODO: Auth
pub async fn delete_task(Path(task_id): Path<String>) -> Result<impl IntoResponse, AppError> {
    validate_task_id(&task_id)?;
    tokio::fs::remove_dir_all(format!("tasks/{task_id}"))
        .await
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => {
                AppError::NotFound(format!("Task '{task_id}' not found"))
            }
            _ => AppError::Internal(e.into()),
        })?;
    Ok(json_response(ResponseCode::Ok))
}

pub async fn get_manifest(Path(task_id): Path<String>) -> Result<impl IntoResponse, AppError> {
    validate_task_id(&task_id)?;
    let task_config = get_task_config(&task_id)?;
    Ok((StatusCode::OK, Json(task_config)))
}

pub async fn get_desc(Path(task_id): Path<String>) -> Result<impl IntoResponse, AppError> {
    validate_task_id(&task_id)?;
    let contents = tokio::fs::read(format!("tasks/{task_id}/desc.pdf"))
        .await
        .map_err(|_| AppError::NotFound(format!("Description for task '{task_id}' not found")))?;

    Ok((
        [
            (header::CONTENT_TYPE, "application/pdf"),
            (header::CONTENT_DISPOSITION, "inline; filename=\"desc.pdf\""),
        ],
        contents,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_task_id_valid() {
        assert!(validate_task_id("a_plus_b").is_ok());
        assert!(validate_task_id("task-123").is_ok());
        assert!(validate_task_id("0").is_ok());
        assert!(validate_task_id("TASK").is_ok());
    }

    #[test]
    fn test_validate_task_id_invalid() {
        assert!(validate_task_id("").is_err());
        assert!(validate_task_id("../etc").is_err());
        assert!(validate_task_id("task/1").is_err());
        assert!(validate_task_id("task\\1").is_err());
        assert!(validate_task_id("task.name").is_err());
        assert!(validate_task_id("task name").is_err());
    }

    #[test]
    fn test_filename_sanitization() {
        let extract_safe_name = |raw: &str| {
            std::path::Path::new(raw)
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
        };

        assert_eq!(
            extract_safe_name("../../evil.txt"),
            Some("evil.txt".to_string())
        );
        assert_eq!(extract_safe_name("/etc/passwd"), Some("passwd".to_string()));
        assert_eq!(
            extract_safe_name("manifest.json"),
            Some("manifest.json".to_string())
        );
        assert_eq!(extract_safe_name(".."), None);
        assert_eq!(extract_safe_name("."), None);
        assert_eq!(extract_safe_name(""), None);
    }

    #[test]
    fn test_safe_extract_zip_valid() {
        let mut buf = Vec::new();
        {
            let mut writer = zip::ZipWriter::new(Cursor::new(&mut buf));
            writer
                .start_file("1.in", zip::write::FileOptions::default())
                .unwrap();
            std::io::Write::write_all(&mut writer, b"1 2\n").unwrap();
            writer.finish().unwrap();
        }

        let temp_dir = std::env::temp_dir().join("test_safe_extract_valid");
        let _ = std::fs::remove_dir_all(&temp_dir);
        assert!(safe_extract_zip(&buf, &temp_dir).is_ok());
        assert_eq!(
            std::fs::read_to_string(temp_dir.join("1.in")).unwrap(),
            "1 2\n"
        );
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_safe_extract_zip_rejects_traversal() {
        let mut buf = Vec::new();
        {
            let mut writer = zip::ZipWriter::new(Cursor::new(&mut buf));
            writer
                .start_file("../evil.txt", zip::write::FileOptions::default())
                .unwrap();
            std::io::Write::write_all(&mut writer, b"evil").unwrap();
            writer.finish().unwrap();
        }

        let temp_dir = std::env::temp_dir().join("test_safe_extract_traversal");
        let _ = std::fs::remove_dir_all(&temp_dir);
        assert!(safe_extract_zip(&buf, &temp_dir).is_err());
        assert!(!temp_dir.exists());
    }
}

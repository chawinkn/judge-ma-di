use std::io::{Cursor, Write};

use judge_ma_di::judge::config::{validate_checker, TaskConfig};
use judge_ma_di::routes::task::{safe_extract_zip, validate_task_id};

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
        writer.write_all(b"1 2\n").unwrap();
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
        writer.write_all(b"evil").unwrap();
        writer.finish().unwrap();
    }

    let temp_dir = std::env::temp_dir().join("test_safe_extract_traversal");
    let _ = std::fs::remove_dir_all(&temp_dir);
    assert!(safe_extract_zip(&buf, &temp_dir).is_err());
    assert!(!temp_dir.exists());
}

#[test]
fn test_manifest_validation_rejects_bad_checker() {
    let bad_manifest = br#"{
        "time_limit": 1.0,
        "memory_limit": 256,
        "checker": "../../../bin/sh",
        "skip": false,
        "full_score": 100,
        "num_testcases": 1,
        "subtasks": []
    }"#;

    let parsed: Result<TaskConfig, _> = serde_json::from_slice(bad_manifest);
    assert!(parsed.is_ok());
    let task_config = parsed.unwrap();
    assert!(validate_checker(&task_config.checker).is_err());
}

#[test]
fn test_manifest_validation_zero_testcases() {
    let manifest_zero = br#"{
        "time_limit": 1.0,
        "memory_limit": 256,
        "checker": "lcmp",
        "skip": false,
        "full_score": 100,
        "num_testcases": 0,
        "subtasks": []
    }"#;

    let parsed: TaskConfig = serde_json::from_slice(manifest_zero).unwrap();
    assert_eq!(parsed.num_testcases, 0);
}

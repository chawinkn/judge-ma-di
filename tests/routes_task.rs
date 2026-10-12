use std::io::{Cursor, Write};

use judge_ma_di::routes::task::safe_extract_zip;

#[test]
fn filename_sanitization_extracts_base_filename() {
    let extract_safe_name = |raw: &str| {
        std::path::Path::new(raw)
            .file_name()
            .and_then(|n| n.to_str())
            .map(str::to_string)
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
fn safe_extract_zip_extracts_valid_archive() {
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
fn safe_extract_zip_rejects_path_traversal() {
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

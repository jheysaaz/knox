use std::path::{Path, PathBuf};

/// Validates that `path` exists and is a non-empty directory path.
pub fn validate_output_dir(path: &Path) -> Result<(), String> {
    if !path.is_dir() {
        return Err("Output directory does not exist".to_string());
    }
    if path.as_os_str().is_empty() {
        return Err("Output directory is empty".to_string());
    }
    Ok(())
}

/// Generates a unique output path under `output_dir`, appending `_cleaned` (and a numeric
/// suffix if a conflict exists) to the input filename.
pub fn safe_output_path(output_dir: &Path, input_path: &Path) -> PathBuf {
    let base = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("document");
    let mut candidate = output_dir.join(format!("{base}_cleaned.pdf"));
    if !candidate.exists() {
        return candidate;
    }

    let mut index = 1;
    loop {
        candidate = output_dir.join(format!("{base}_cleaned_{index}.pdf"));
        if !candidate.exists() {
            return candidate;
        }
        index += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn validate_output_dir_ok() {
        let dir = std::env::temp_dir().join("knox_test_validate_ok");
        fs::create_dir_all(&dir).unwrap();
        assert!(validate_output_dir(&dir).is_ok());
        fs::remove_dir(&dir).unwrap();
    }

    #[test]
    fn validate_output_dir_not_exists() {
        let dir = std::env::temp_dir().join("knox_test_does_not_exist_12345");
        let result = validate_output_dir(&dir);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[test]
    fn safe_output_path_no_conflict() {
        let dir = std::env::temp_dir().join("knox_test_no_conflict");
        fs::create_dir_all(&dir).unwrap();
        let input = Path::new("/path/to/document.pdf");
        let result = safe_output_path(&dir, input);
        assert_eq!(result, dir.join("document_cleaned.pdf"));
        fs::remove_dir(&dir).unwrap();
    }

    #[test]
    fn safe_output_path_one_conflict() {
        let dir = std::env::temp_dir().join("knox_test_one_conflict");
        fs::create_dir_all(&dir).unwrap();
        let existing = dir.join("document_cleaned.pdf");
        fs::write(&existing, "exists").unwrap();
        let input = Path::new("/path/to/document.pdf");
        let result = safe_output_path(&dir, input);
        assert_eq!(result, dir.join("document_cleaned_1.pdf"));
        fs::remove_file(&existing).unwrap();
        fs::remove_dir(&dir).unwrap();
    }

    #[test]
    fn safe_output_path_fallback_stem() {
        let dir = std::env::temp_dir().join("knox_test_fallback");
        fs::create_dir_all(&dir).unwrap();
        // Path with no file_stem (empty filename at end)
        let input = Path::new("/path/");
        let result = safe_output_path(&dir, input);
        // file_stem() returns None for path ending in / -> fallback to "document"
        assert!(result.to_string_lossy().contains("_cleaned.pdf"));
        fs::remove_dir(&dir).unwrap();
    }

    #[test]
    fn safe_output_path_multiple_conflicts() {
        let dir = std::env::temp_dir().join("knox_test_multi_conflict");
        fs::create_dir_all(&dir).unwrap();
        // Create document_cleaned.pdf to force numbered fallback
        let base = dir.join("document_cleaned.pdf");
        fs::write(&base, "exists").unwrap();
        for i in 1..=5 {
            let f = dir.join(format!("document_cleaned_{i}.pdf"));
            fs::write(&f, "exists").unwrap();
        }
        let input = Path::new("/path/to/document.pdf");
        let result = safe_output_path(&dir, input);
        assert_eq!(result, dir.join("document_cleaned_6.pdf"));
        // Cleanup
        fs::remove_file(&base).unwrap();
        for i in 1..=6 {
            let _ = fs::remove_file(dir.join(format!("document_cleaned_{i}.pdf")));
        }
        fs::remove_dir(&dir).unwrap();
    }

    #[test]
    fn validate_output_dir_rejects_file_path() {
        let dir = std::env::temp_dir().join("knox_test_file_not_dir");
        fs::write(&dir, "i am a file not a directory").unwrap();
        let result = validate_output_dir(&dir);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("does not exist") || msg.contains("not a directory"));
        fs::remove_file(&dir).unwrap();
    }

    #[test]
    fn safe_output_path_handles_path_traversal_input() {
        let dir = std::env::temp_dir().join("knox_test_traversal");
        fs::create_dir_all(&dir).unwrap();
        // Input path with "../" component should not escape the output directory
        let input = Path::new("../../etc/passwd.pdf");
        let result = safe_output_path(&dir, input);
        let result_str = result.to_string_lossy().to_string();
        // Output should be inside the output directory, not traversing out
        assert!(
            result_str.contains("_cleaned.pdf"),
            "result should contain _cleaned suffix: {result_str}"
        );
        assert!(
            result_str.starts_with(dir.to_string_lossy().as_ref()),
            "result should be inside output dir: {result_str}"
        );
        fs::remove_dir(&dir).unwrap();
    }

    #[test]
    fn safe_output_path_handles_no_extension() {
        let dir = std::env::temp_dir().join("knox_test_no_ext");
        fs::create_dir_all(&dir).unwrap();
        let input = Path::new("document");
        let result = safe_output_path(&dir, input);
        assert!(result.to_string_lossy().contains("document_cleaned"));
        fs::remove_dir(&dir).unwrap();
    }

    #[test]
    fn safe_output_path_handles_unicode() {
        let dir = std::env::temp_dir().join("knox_test_unicode");
        fs::create_dir_all(&dir).unwrap();
        let input = Path::new("café_100%.pdf");
        let result = safe_output_path(&dir, input);
        assert!(result.to_string_lossy().contains("_cleaned.pdf"));
        fs::remove_dir(&dir).unwrap();
    }
}

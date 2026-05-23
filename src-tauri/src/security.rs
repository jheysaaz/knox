use std::path::{Path, PathBuf};

pub fn validate_output_dir(path: &Path) -> Result<(), String> {
    if !path.is_dir() {
        return Err("Output directory does not exist".to_string());
    }
    if path.as_os_str().is_empty() {
        return Err("Output directory is empty".to_string());
    }
    Ok(())
}

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

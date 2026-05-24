# Security Module Spec

## Functions

### `validate_output_dir(path: &Path) -> Result<(), String>`
- Returns Ok if path exists and is a directory
- Returns Err("Output directory does not exist") if not a directory
- Returns Err("Output directory is empty") if path is empty

### `safe_output_path(output_dir: &Path, input_path: &Path) -> PathBuf`
- Generates output path: `{output_dir}/{stem}_cleaned.pdf`
- If file already exists, appends index: `{stem}_cleaned_{n}.pdf`
- Falls back to "document" if input has no file stem

## Acceptance Criteria
- Valid directory returns Ok
- Non-existent directory returns Err
- Empty path returns Err
- No conflict: returns `{stem}_cleaned.pdf`
- One conflict: returns `{stem}_cleaned_1.pdf`
- Multiple conflicts: increments index until unique
- Non-utf8 path: uses "document" as fallback

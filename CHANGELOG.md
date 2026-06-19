# Changelog

## [Unreleased]

### Added
- Save/load named custom profiles (localStorage)
- Right panel redesign: counter badges, tab-bar action buttons
- Password-protected PDF support with per-file dialog
- macOS liquid glass icon via tauri-liquid-icon
- Collapsible profile sections in advanced options
- Responsive layout (lg/xl breakpoints)
- Cmd+Enter shortcut to start processing
- `check_file_encrypted` Rust command

### Changed
- Right panel: Pause/Clear buttons moved from card headers to tab bar
- Right panel: Badges show queue/history entry count (max 99)
- Balanced profile defaults: compression changed from CCITT to Flate
- Profile detection uses deep-equality for language arrays
- `retryFilePaths` ref removed from useQueue (dead code)
- History now strips password before storing entries

### Fixed
- History not loading on app start
- Timestamp display off by factor 1000
- "Completed" with no output when all pages have existing text
- Focus ring clipping on left scroll container
- `check_file_encrypted` always returned `encrypted: false`
- `load_document` never passed password to lopdf
- Wrong password stayed cached after error

### Removed
- Linux support (CI, build.rs, pdfium script, docs)
- macOS Intel (x86_64-apple-darwin) support
- iOS/android icon directories
- Unused `retryFilePaths` ref

## [0.1.0-alpha] — Unreleased

### Added
- Batch OCR with per-file progress tracking
- Rust-native pipeline via tesseract-sys FFI
- Image preprocessing: denoise, binarize (Otsu/Bradley-Roth/Fixed), deskew (Radon/Hough)
- CCITT Group 4 and FlateDecode compression
- PDF/A compliance metadata injection
- 4 profiles: Balanced, Max Compression, High Fidelity, Custom
- Queue management: enqueue, start, pause, remove, clear
- History persistence (100 entries, JSON)
- Session activity log with save-to-file
- Password-protected PDF detection
- Tauri v2 desktop shell with React 19 + Tailwind CSS 4 + shadcn/ui
- macOS DMG and Windows MSI/NSIS bundling

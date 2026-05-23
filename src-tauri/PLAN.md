# OCR Engine Implementation Plan

This plan reflects the finalized constraints for the Rust-native OCR pipeline, including static linking, CCITT G4 compression, Radon deskewing, panic isolation, and resource throttling. All tasks below are required and are implemented in the current codebase changes.

## Goals and Non-Negotiables

- Fully static linking of `libtesseract` and `libleptonica` into the Rust binary.
- Hard resource throttling: custom Rayon pool with `available_parallelism() - 2` threads and a memory backpressure semaphore controlling concurrent in-memory files.
- Panic isolation at all FFI boundaries using `catch_unwind`.
- Deterministic memory management with `Drop` for all FFI pointers.
- Robust fault tolerance: corrupt pages/files are skipped and logged; no pool poison or process termination.

## Task List (Detailed)

### 1) Build and Link Strategy

- Update `src-tauri/build.rs` to enforce static linking:
  - Read `TARGET` and map to `third_party/native/[target-triple]`.
  - Emit `cargo:rustc-link-search=native=...`.
  - Emit `cargo:rustc-link-lib=static=tesseract` and `static=lept`.
  - Emit platform-specific system libs (Windows: `advapi32`, `user32`, `gdi32`; macOS: `c++`, `z`; Linux: `stdc++`, `z`).
- Document required archive layout:
  - `third_party/native/x86_64-pc-windows-msvc/{tesseract.lib, lept.lib}`
  - `third_party/native/aarch64-apple-darwin/libtesseract.a libliblept.a`
  - `third_party/native/x86_64-apple-darwin/libtesseract.a libliblept.a`
  - `third_party/native/x86_64-unknown-linux-gnu/libtesseract.a liblept.a`

### 2) Cargo Dependencies

- Add crates:
  - `tesseract-sys`, `leptonica-sys` for statically linked OCR.
  - `lopdf` for PDF parsing + reconstruction.
  - `image`, `imageproc` for preprocessing.
  - `rayon` for CPU-bound parallelism.
  - `tokio` for async ingestion and backpressure.
  - `tracing`, `tracing-appender`, `tracing-subscriber` for structured logging.
  - `thiserror` for error hierarchy.
  - `futures` for async test assertions.

### 3) Module Architecture

- `src-tauri/src/ocr_engine/`:
  - `config.rs`: dynamic semaphore default.
  - `runtime.rs`: custom Rayon pool + semaphore.
  - `ingest.rs`: bounded channels and file ingestion.
  - `pdf.rs`: PDF parse, image extract/replace, CCITT G4 encode, and final compression.
  - `image.rs`: Otsu threshold, Radon deskew, denoise, 1-bit conversion.
  - `ocr.rs`: safe FFI wrapper with `Drop` and panic isolation.
  - `progress.rs`: `PipelineProgress` emitter to Tauri.
  - `types.rs`: shared config and progress schema.

### 4) Resource Throttling

- Build Rayon pool with `max(1, available_parallelism() - 2)` threads.
- Allocate `tokio::sync::Semaphore` with capacity:
  - `ProcessingConfig.max_concurrent_files` when present.
  - Otherwise default to `max(1, available_parallelism()/2)`.
- Tie permits to file processing lifetime (release on task completion).

### 5) Ingest and Backpressure

- Use `tokio::sync::mpsc::channel` with bounded capacity.
- Prevent producer overruns by awaiting sends.
- Support queue-driven processing with strict concurrency limits.

### 6) PDF Parsing and Reconstruction

- Parse with `lopdf::Document`.
- Locate page XObjects and extract raster images.
- For each processed page image:
  - Replace page XObject stream.
  - Maintain dictionary references correctly.
- Call `document.compress()` before saving.

### 7) Image Processing

- Apply median denoise.
- Otsu binarization.
- Radon-based deskew with angle search on downscaled image.
- Morphological open-close to reduce noise.
- Convert to 1-bit bitonal when safe.

### 8) Compression Strategy

- For binarized pages:
  - Encode as 1-bit bitonal with CCITT Group 4 (`CCITTFaxDecode`, `K=-1`).
- Fallback:
  - Use FlateDecode for non-bitonal pages.

### 9) OCR Wrapper

- Safe wrapper over `tesseract-sys`:
  - `TessBaseAPICreate`, `Init3`, `SetImage`, `Recognize`, `GetUTF8Text`.
  - Free buffers with `TessDeleteText`.
  - `Drop` calls `End` and `Delete`.
- Wrap all FFI calls with `catch_unwind` to isolate panics.

### 10) Progress Reporting

- Emit `PipelineProgress` over Tauri event `pipeline-progress`.
- State machine: `Processing → Ocr → Compressing → Completed/Failed`.
- Track averages and totals with atomic counters.

### 11) Error Handling

- `PipelineError` (thiserror):
  - `Io`, `PdfParse`, `ImageProcessing`, `FfiOcr`, `Compression`, `Channel`, `Progress`, `PanicRecovered`.
- Ensure errors do not abort the global pool.

### 12) Tests

- `Send + Sync` validation for runtime resources.
- Bounded channel backpressure test.
- Panic recovery test for FFI guard.
- Default semaphore capacity tests.

## Next Practical Steps

1) Provide prebuilt static archives in `third_party/native/[target-triple]`.
2) Bundle `tessdata_fast` (eng + spa) in the Tauri resource directory.
3) Wire frontend settings for `ProcessingConfig.max_concurrent_files`.
4) Run `cargo test` in `src-tauri`.
5) Perform end-to-end OCR run with corrupted PDFs to validate panic isolation.

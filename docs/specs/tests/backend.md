# Backend Test Strategy

## Framework
- **Runner**: `cargo test` (built-in)
- **Tests**: Inline `#[cfg(test)]` modules + `tests.rs` for integration

## Test Organization
Tests are placed in two locations:
1. **Inline `#[cfg(test)]` modules** in each source file for unit tests
2. **`tests.rs`** for integration tests involving multiple modules

## Module Coverage

| Module | Test Type | What to Test |
|---|---|---|
| `security.rs` | Unit | Path validation, output path dedup |
| `config.rs` | Unit | Effective concurrency calculation |
| `runtime.rs` | Unit | Pool + semaphore construction |
| `ingest.rs` | Unit | Channel send/close behavior |
| `progress.rs` | Unit | Counter increments, avg computation, div-by-zero |
| `image.rs` | Unit | Denoise, binarize, deskew, morphology, bitonal packing, DPI |
| `pdf.rs` | Unit | Encode functions, stream dict fields, decode_image, page_has_text, finalize |
| `ocr.rs` | Unit | Panic isolation (guard_unwind), string encoding |
| `tests.rs` | Integration | Queue lifecycle, history, runner config, file metadata |

## Test Principles
- No external dependencies (no real Tesseract, no real PDFs for unit tests)
- Use in-memory buffers and synthetic images
- Integration tests test queue logic only (not actual OCR processing)
- FFI-dependent tests are gated behind feature flags when needed

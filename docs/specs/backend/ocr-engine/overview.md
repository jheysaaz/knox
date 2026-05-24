# OCR Engine Overview

## Architecture
```
IngestItem → process_files() → for each file:
  load_document() → extract_page_images()
  → for each page: downsample → preprocess → OCR → encode
  → replace_page_images() → finalize() → save
```

## Pipeline Stages
1. **Load**: Parse PDF via lopdf, extract page image streams
2. **Preprocess**: Denoise → Binarize → Deskew → Morphology → Bitonal
3. **OCR**: Tesseract FFI (panics isolated via catch_unwind)
4. **Encode**: CCITT G4 (bitonal) or FlateDecode (grayscale)
5. **Save**: Replace image streams, compress, write to disk

## Module Responsibilities
| Module | Role |
|---|---|
| `config.rs` | Semaphore capacity calculation |
| `runtime.rs` | Rayon pool + file semaphore |
| `ingest.rs` | Bounded channel ingestion |
| `engine.rs` | Pipeline orchestrator |
| `image.rs` | Image preprocessing |
| `ocr.rs` | Tesseract FFI wrapper |
| `pdf.rs` | PDF manipulation |
| `progress.rs` | Progress tracking + events |
| `types.rs` | Shared enums/structs |
| `error.rs` | Error types |

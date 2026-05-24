# OCR Engine Overview

## Architecture
```
IngestItem → process_files() → for each file:
  load_document() → read pdf_bytes
  → for each page:
      try PdfiumEngine::render_page() → if Ok(Some(img)) → use it
      else → extract_lopdf_page() (fallback)
  → for each page: downsample → preprocess → OCR → encode
  → replace_page_images() → finalize() → save
```

## Pipeline Stages
1. **Load**: Parse PDF via lopdf for page count
2. **Render**: Full-page rasterization via pdfium-render (handles AcroForms, all filters)
3. **Fallback**: lopdf XObject image extraction when pdfium unavailable
4. **Preprocess**: Denoise → Binarize → Deskew → Morphology → Bitonal
5. **OCR**: Tesseract FFI (panics isolated via catch_unwind)
6. **Encode**: CCITT G4 (bitonal) or FlateDecode (grayscale)
7. **Save**: Replace image streams, compress, write to disk

## Module Responsibilities
| Module | Role |
|---|---|
| `config.rs` | Semaphore capacity calculation |
| `runtime.rs` | Rayon pool + file semaphore |
| `ingest.rs` | Bounded channel ingestion |
| `engine.rs` | Pipeline orchestrator |
| `image.rs` | Image preprocessing |
| `ocr.rs` | Tesseract FFI wrapper |
| `pdf.rs` | PDF manipulation (load, encode, save) |
| `render.rs` | PDFium hybrid extraction |
| `progress.rs` | Progress tracking + events |
| `types.rs` | Shared enums/structs |
| `error.rs` | Error types |

# Render Module Spec — PDFium Hybrid Extraction

## Purpose

Provide full-page rasterization via pdfium-render as an alternative to lopdf's XObject image extraction. Pdfium handles AcroForms, CCITT, JBIG2, and all filter types natively — files that lopdf cannot decode fall through to pdfium.

## Architecture

```
input.pdf ──→ load_document (lopdf, for page count)
            ──→ pdf_bytes (raw fs::read)
                  ↓
            PdfiumEngine::render_page(&bytes, idx, dpi)
                  ↓
            Ok(Some(GrayImage)) ──→ OCR pipeline (unchanged)
            Ok(None) ──→ fallback to lopdf extraction
            Err(e)  ──→ log warning → fallback to lopdf
```

## Functions

### `PdfiumEngine::new(lib_path: &str) -> Self`

- Calls `Pdfium::bind_to_library(lib_path)` wrapped in `catch_unwind`.
- On failure: log warning, store `inner = None`.
- On success: store `inner = Some(Pdfium)`.
- **Panic isolation**: any panic inside pdfium binding is caught and converted to fallback.

### `PdfiumEngine::render_page(&self, doc_bytes: &[u8], page_index: u32, dpi: u16) -> Result<Option<GrayImage>>`

- If `inner` is `None`: return `Ok(None)` (silent fallback).
- Call `Pdfium::load_document_from_bytes(doc_bytes, None)`.
- Get page by `page_index`; get page size in points.
- Compute pixel dimensions: `px = (pt * dpi / 72.0).round()`.
- Create `PdfRenderConfig` with `render_form_data(true).render_annotations(true)`.
- Render to `PdfBitmapFormat::Gray` (8bpp grayscale).
- Extract raw bytes; wrap in `GrayImage::from_raw()`.
- Handle errors gracefully: log warning, return `Ok(None)` for fallback.

### `resolve_pdfium_path` (in lib.rs)

Search order (first match wins):

1. `PDFIUM_LIB_PATH` environment variable
2. Bundled resource: `resources/pdfium/libpdfium.dylib` (macOS), `libpdfium.so` (Linux), `pdfium.dll` (Windows)
3. System paths: `/opt/homebrew/lib/libpdfium.dylib`, `/usr/local/lib/libpdfium.so`

Returns `None` if nothing found (graceful fallback to lopdf).

## Acceptance Criteria

- `PdfiumEngine::new("")` / invalid path → `inner` is `None`, no panic
- `PdfiumEngine::new` with valid dylib → `inner` is `Some`
- `render_page` with `inner = None` → returns `Ok(None)`
- `render_page` with valid PDF + valid dylib → returns `Ok(Some(GrayImage))` with correct dimensions
- `render_page` renders AcroForm field content (not blank)
- `render_page` handles corrupt bytes without panic → returns `Err` → engine falls back
- `PdfiumEngine` implements `Send + Sync`

## Boundary Conditions

| Edge Case | Handling |
|---|---|
| Dylib path invalid | `inner = None`, silent fallback |
| Dylib corrupt/wrong arch | `catch_unwind` → `inner = None` |
| PDF bytes truncated/corrupt | `Err(PipelineError::Pdfium(...))` |
| Page index out of bounds | `Err(PipelineError::Pdfium(...))` |
| 0x0 page dimensions | `GrayImage::from_raw` returns `None` → log warning → `Ok(None)` |
| DPI too high (> 1200) | Clamp to 1200 to prevent OOM |
| Different platform | Binary name differs per OS (handled in path resolution + bundling) |

# OCR Module Spec

## Struct: `TessApi`
Safe wrapper around tesseract-sys FFI with automatic cleanup via Drop.

## Constructor
- `TessApi::new(tessdata_path, languages)` → Initialize Tesseract API
  - Calls TessBaseAPICreate, TessBaseAPIInit3
  - Returns FfiOcr error on init failure
  - Returns PanicRecovered error if FFI call panics

## Methods
- `set_image_bytes(data, width, height, bpp, bpl)` → Set image + run recognition
  - Calls TessBaseAPISetImage + TessBaseAPIRecognize
- `get_text() -> String` → Get recognized text
  - Calls TessBaseAPIGetUTF8Text
  - Frees buffer via TessDeleteText
- `set_page_seg_mode(mode)` → Set PSM

## Panic Isolation
All FFI calls wrapped in `guard_unwind(fn)`:
- Calls catch_unwind(AssertUnwindSafe(fn))
- Returns PanicRecovered error on panic
- Prevents Tesseract crashes from taking down the process

## Cleanup
- Drop impl calls TessBaseAPIEnd + TessBaseAPIDelete
- Ensures no memory leaks even after errors

## Acceptance Criteria
- guard_unwind catches panics from FFI calls
- to_cstring properly encodes strings
- Null TessBaseAPICreate returns FfiOcr error
- Drop is called even when early return occurs

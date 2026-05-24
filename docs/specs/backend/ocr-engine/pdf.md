# PDF Module Spec

## Functions

### `load_document(path) -> Document`
- Wraps lopdf::Document::load
- Returns PdfParse error on failure

### `extract_page_images(doc, existing_text) -> Vec<PdfPageInfo>`
- Iterates pages, extracts XObject images
- Skips pages with text content if existing_text=Skip
- Decodes streams (FlateDecode, DCTDecode, JBIG2Decode, raw)

### `page_has_text(doc, page_id) -> bool`
- Parses page content stream for text operators: Tj, TJ, ', "

### `replace_page_images(doc, replacements)`
- For each page with a replacement stream, replaces the first Image XObject

### `encode_ccitt_g4(width, height, bitonal_data) -> Stream`
- Creates XObject stream with CCITTFaxDecode filter
- DecodeParms: K=-1, Columns=width, Rows=height, BlackIs1=true

### `encode_flate(width, height, data) -> Stream`
- Creates XObject stream with FlateDecode filter
- DeviceGray, 8 bits per component

### `finalize(doc, output_path, enforce_pdfa)`
- Optionally adds PDF/A-2b metadata (Producer, Creator, XMP)
- Calls doc.compress() and doc.save()

### `decode_stream_image(stream, doc) -> GrayImage`
- Supports: FlateDecode, DCTDecode (JPEG), JBIG2Decode, raw (no filter)

## Acceptance Criteria
- load_document fails on non-existent path
- encode_ccitt_g4 produces correct stream dict (K=-1, CCITTFaxDecode)
- encode_flate produces correct stream dict (FlateDecode, 8-bit)
- decode_stream_image decodes valid streams for each filter
- finalize with PDF/A adds XMP metadata
- page_has_text detects Tj/TJ operators
- page_has_text returns false for no-text pages

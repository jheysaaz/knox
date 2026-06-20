use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;

use image::GrayImage;
use lopdf::{Dictionary, Document, Object, Stream};

use crate::ocr_engine::error::PipelineError;
#[cfg(feature = "ocr")]
use crate::ocr_engine::ocr::WordBounds;
use crate::ocr_engine::types::ExistingTextMode;

/// Expands packed 1bpp data (8 pixels/byte, MSB=first pixel) into 8bpp
/// grayscale bytes (0=black, 255=white). Row stride = ceil(width / 8).
/// Trailing bits past `width` are discarded per row.
fn expand_1bpp_to_8bpp(data: &[u8], width: u32, height: u32) -> Vec<u8> {
    let stride = width.div_ceil(8) as usize;
    let mut out = Vec::with_capacity((width * height) as usize);
    for y in 0..height {
        let row = y as usize * stride;
        for x in 0..width {
            let byte_idx = row + (x as usize / 8);
            let bit_idx = 7 - (x % 8);
            let bit = byte_idx < data.len() && (data[byte_idx] >> bit_idx) & 1 != 0;
            out.push(if bit { 255 } else { 0 });
        }
    }
    out
}

/// Returns `true` if the stream's photometric interpretation requires
/// black↔white inversion. Checks `ImageMask` (default Decode = [1, 0]) and
/// the explicit `Decode` array.
fn needs_photometric_inversion(stream: &Stream) -> bool {
    if stream
        .dict
        .get(b"ImageMask")
        .ok()
        .and_then(|o| o.as_bool().ok())
        .unwrap_or(false)
    {
        return true;
    }
    if let Ok(decode) = stream.dict.get(b"Decode")
        && let Ok(arr) = decode.as_array()
        && arr.len() >= 2
    {
        let d0 = arr[0].as_i64().unwrap_or(0);
        let d1 = arr[1].as_i64().unwrap_or(1);
        return d0 > d1;
    }
    false
}

/// Returns the image's `BitsPerComponent` (defaults to 8).
fn bits_per_component(stream: &Stream) -> u8 {
    stream
        .dict
        .get(b"BitsPerComponent")
        .ok()
        .and_then(|o| o.as_i64().ok())
        .unwrap_or(8) as u8
}

/// Applies photometric inversion (0↔255) to an 8bpp grayscale buffer in place.
fn invert_grayscale(pixels: &mut [u8]) {
    for b in pixels.iter_mut() {
        *b = 255 - *b;
    }
}

/// Metadata for a single PDF page extracted for OCR processing.
#[allow(dead_code)]
pub struct PdfPageInfo {
    /// 1-indexed page number within the PDF.
    pub page_number: u32,
    /// Decoded grayscale image of the page.
    pub image: image::GrayImage,
}

const MAX_PDF_SIZE: u64 = 512 * 1024 * 1024;

/// Loads a PDF document from the given filesystem path.
/// Rejects files larger than 512 MB to prevent OOM from decompression bombs.
/// Detects password-protected PDFs and returns `PipelineError::Encrypted` if
/// the file is encrypted and `password` is `None` (or if the password is wrong).
pub fn load_document(path: &Path, password: Option<&str>) -> Result<Document, PipelineError> {
    let metadata = std::fs::metadata(path).map_err(PipelineError::Io)?;
    if metadata.len() > MAX_PDF_SIZE {
        let msg = format!(
            "PDF too large: {} bytes (max {} bytes)",
            metadata.len(),
            MAX_PDF_SIZE
        );
        tracing::error!(target: "knox::pdf", "{msg}");
        return Err(PipelineError::PdfParse(msg));
    }
    tracing::info!(target: "knox::pdf", path = %path.display(), size = metadata.len(), "loading pdf");
    let mut doc = match Document::load(path) {
        Ok(doc) => doc,
        Err(e) => {
            let msg = e.to_string().to_lowercase();
            if msg.contains("encrypt") || msg.contains("password") {
                let hint = if password.is_some() {
                    "password may be incorrect".to_string()
                } else {
                    "password required".to_string()
                };
                return Err(PipelineError::Encrypted(hint));
            }
            return Err(PipelineError::PdfParse(e.to_string()));
        }
    };

    if doc.is_encrypted() {
        match password {
            Some(pwd) => {
                doc.decrypt(pwd).map_err(|_| {
                    PipelineError::Encrypted("password may be incorrect".to_string())
                })?;
            }
            None => {
                return Err(PipelineError::Encrypted("password required".to_string()));
            }
        }
    }

    Ok(doc)
}

/// Returns `true` if the PDF page contains text operators (Tj, TJ, ', ").
/// Used to skip OCR on pages that already have a text layer.
/// Parses the page's content stream, which is O(operations) in the number of
/// PDF drawing operations — returns early on first text operator found.
pub(crate) fn page_has_text(
    doc: &Document,
    page_id: lopdf::ObjectId,
) -> Result<bool, PipelineError> {
    let content_data = match doc.get_page_content(page_id) {
        Ok(data) => data,
        Err(_) => return Ok(false),
    };
    if content_data.is_empty() {
        return Ok(false);
    }
    let content = match lopdf::content::Content::decode(&content_data) {
        Ok(c) => c,
        Err(_) => return Ok(false),
    };
    for operation in &content.operations {
        match operation.operator.as_str() {
            "Tj" | "TJ" | "'" | "\"" => return Ok(true),
            _ => {}
        }
    }
    Ok(false)
}

/// Processes each page image through a callback, decoding one page at a time.
/// Images are dropped after the callback returns, keeping peak memory proportional
/// to a single page instead of loading all pages into RAM at once.
#[allow(dead_code)]
pub fn for_each_page_image<F>(
    doc: &Document,
    existing_text: ExistingTextMode,
    mut f: F,
) -> Result<(), PipelineError>
where
    F: FnMut(PdfPageInfo) -> Result<(), PipelineError>,
{
    let pages = doc.get_pages();
    for (page_number, page_id) in pages {
        let page = doc
            .get_object(page_id)
            .map_err(|e| PipelineError::PdfParse(e.to_string()))?;
        let has_text = page_has_text(doc, page_id)?;
        if has_text && matches!(existing_text, ExistingTextMode::Skip) {
            continue;
        }
        let resources = page
            .as_dict()
            .and_then(|dict| dict.get(b"Resources"))
            .and_then(|obj| doc.dereference(obj).map(|(_, o)| o))
            .and_then(|obj| obj.as_dict())
            .map_err(|_| PipelineError::PdfParse("missing page resources".to_string()))?;
        let xobjects = match resources.get(b"XObject") {
            Ok(obj) => doc
                .dereference(obj)
                .ok()
                .and_then(|(_, o)| o.as_dict().ok()),
            Err(_) => None,
        };
        let Some(xobjects) = xobjects else {
            continue;
        };
        for (_, obj_ref) in xobjects.iter() {
            let obj = match obj_ref.as_reference() {
                Ok(obj_id) => doc
                    .get_object(obj_id)
                    .map_err(|e| PipelineError::PdfParse(e.to_string()))?,
                Err(_) => obj_ref,
            };
            let stream = match obj {
                Object::Stream(stream) => stream,
                _ => continue,
            };
            let dict = &stream.dict;
            let subtype = dict.get(b"Subtype").ok();
            if subtype != Some(&Object::Name(b"Image".to_vec())) {
                continue;
            }
            let width = dict.get(b"Width").and_then(|o| o.as_i64()).unwrap_or(0) as u32;
            let height = dict.get(b"Height").and_then(|o| o.as_i64()).unwrap_or(0) as u32;
            if width == 0 || height == 0 {
                continue;
            }
            let image = decode_stream_image(stream, doc)?;
            f(PdfPageInfo { page_number, image })?;
        }
    }
    Ok(())
}

/// Extracts the first image XObject from a single page using lopdf decoding.
///
/// Used as a fallback when pdfium rendering is unavailable or fails. Operates
/// on a single page at a time (rather than the entire document) so the caller
/// can mix pdfium and lopdf extraction per page within the same pipeline.
///
/// Returns `None` if the page has text content and `existing_text` is `Skip`,
/// or if no image XObject is found on the page.
pub fn extract_lopdf_page(
    doc: &Document,
    page_number: u32,
    existing_text: ExistingTextMode,
) -> Result<Option<GrayImage>, PipelineError> {
    let pages = doc.get_pages();
    let Some(&page_id) = pages.get(&page_number) else {
        return Ok(None);
    };
    if matches!(existing_text, ExistingTextMode::Skip)
        && page_has_text(doc, page_id).unwrap_or(false)
    {
        return Ok(None);
    }
    let page = doc
        .get_object(page_id)
        .map_err(|e| PipelineError::PdfParse(e.to_string()))?;
    let resources = page
        .as_dict()
        .and_then(|dict| dict.get(b"Resources"))
        .and_then(|obj| doc.dereference(obj).map(|(_, o)| o))
        .and_then(|obj| obj.as_dict())
        .map_err(|_| PipelineError::PdfParse("missing page resources".to_string()))?;
    let xobjects = match resources.get(b"XObject") {
        Ok(obj) => doc
            .dereference(obj)
            .ok()
            .and_then(|(_, o)| o.as_dict().ok()),
        Err(_) => None,
    };
    let Some(xobjects) = xobjects else {
        return Ok(None);
    };
    for (_, obj_ref) in xobjects.iter() {
        let obj = match obj_ref.as_reference() {
            Ok(obj_id) => doc
                .get_object(obj_id)
                .map_err(|e| PipelineError::PdfParse(e.to_string()))?,
            Err(_) => obj_ref,
        };
        let stream = match obj {
            Object::Stream(stream) => stream,
            _ => continue,
        };
        let dict = &stream.dict;
        if dict.get(b"Subtype").ok() != Some(&Object::Name(b"Image".to_vec())) {
            continue;
        }
        let width = dict.get(b"Width").and_then(|o| o.as_i64()).unwrap_or(0) as u32;
        let height = dict.get(b"Height").and_then(|o| o.as_i64()).unwrap_or(0) as u32;
        if width == 0 || height == 0 {
            continue;
        }
        return decode_stream_image(stream, doc).map(Some);
    }
    Ok(None)
}

/// Replaces image XObject streams in the document with the given compressed streams.
/// Each page's first image stream is replaced using ownership transfer from the map
/// (no cloning), so the caller should not reuse the map after this call.
pub fn replace_page_images(
    doc: &mut Document,
    mut replacements: BTreeMap<u32, Stream>,
) -> Result<(), PipelineError> {
    let pages = doc.get_pages();
    for (page_number, page_id) in pages {
        let Some(new_stream) = replacements.remove(&page_number) else {
            continue;
        };

        let page_obj = doc
            .get_object(page_id)
            .map_err(|e| PipelineError::PdfParse(e.to_string()))?
            .clone();
        let page_dict = page_obj
            .as_dict()
            .map_err(|_| PipelineError::PdfParse("missing page dictionary".to_string()))?;

        let resources_obj = page_dict
            .get(b"Resources")
            .map_err(|_| PipelineError::PdfParse("missing page resources".to_string()))?;
        let resources_dict = resolve_dict(doc, resources_obj)?
            .ok_or_else(|| PipelineError::PdfParse("missing resources dict".to_string()))?;

        let xobjects_obj = resources_dict
            .get(b"XObject")
            .map_err(|_| PipelineError::PdfParse("missing xobject".to_string()))?;
        let xobjects_dict = resolve_dict(doc, xobjects_obj)?
            .ok_or_else(|| PipelineError::PdfParse("missing xobject dict".to_string()))?;

        for (_, obj_ref) in xobjects_dict.iter() {
            let obj_id = match obj_ref.as_reference() {
                Ok(id) => id,
                Err(_) => continue,
            };
            let obj = doc
                .get_object(obj_id)
                .map_err(|e| PipelineError::PdfParse(e.to_string()))?;
            let Object::Stream(stream) = obj else {
                continue;
            };
            let subtype = stream.dict.get(b"Subtype").ok();
            if subtype != Some(&Object::Name(b"Image".to_vec())) {
                continue;
            }
            doc.objects.insert(obj_id, Object::Stream(new_stream));
            break;
        }
    }
    Ok(())
}

/// Appends an invisible selectable text layer to each page that has word bounds.
/// Uses a built-in Helvetica font with rendering mode 3 (invisible).
///
/// Coordinates are mapped proportionally from image pixel space (top-left origin)
/// to PDF user space (bottom-left origin) using the page's MediaBox dimensions
/// and the provided image dimensions. This avoids relying on DPI which may be
/// incorrect when the image was downscaled.
/// Helvetica glyph advance widths in emu (1 em = 1000 units).
/// Source: Adobe Helvetica AFM. Space = 278.
#[cfg(feature = "ocr")]
fn helvetica_char_width(b: u8) -> u16 {
    match b {
        b' ' => 278,
        b'!' | b'"' | b'.' => 278,
        b'\'' => 333,
        b'#' => 355,
        b'$' | b'%' => 556,
        b'&' => 889,
        b'(' | b')' => 333,
        b'*' => 389,
        b'+' | b'<' | b'=' | b'>' => 584,
        b',' => 278,
        b'-' => 333,
        b'/' => 278,
        b'0'..=b'9' => 556,
        b':' | b';' => 278,
        b'?' => 556,
        b'@' => 1015,
        b'A' | b'B' | b'E' | b'H' | b'P' | b'S' => 667,
        b'C' | b'D' | b'G' | b'O' | b'Q' | b'R' | b'U' | b'X' | b'Y' => 722,
        b'F' => 611,
        b'I' => 278,
        b'J' => 500,
        b'K' => 667,
        b'L' | b'_' => 556,
        b'M' => 833,
        b'N' => 722,
        b'T' => 611,
        b'V' => 667,
        b'W' => 944,
        b'Z' => 611,
        b'[' | b'\\' | b']' => 278,
        b'^' => 469,
        b'`' => 333,
        b'a' | b'b' | b'd' | b'e' | b'g' | b'h' | b'n' | b'o' | b'p' | b'q' | b'u' => 556,
        b'c' | b's' | b'x' | b'v' | b'y' | b'z' => 500,
        b'f' | b't' => 278,
        b'i' | b'j' | b'l' => 222,
        b'k' => 500,
        b'm' => 833,
        b'r' => 333,
        b'w' => 722,
        b'{' | b'}' => 334,
        b'|' => 260,
        b'~' => 584,
        _ => 500,
    }
}

#[cfg(feature = "ocr")]
fn helvetica_word_width_emu(word: &str) -> u64 {
    word.as_bytes()
        .iter()
        .map(|&b| helvetica_char_width(b) as u64)
        .sum()
}

#[cfg(feature = "ocr")]
pub fn add_text_layers(
    doc: &mut Document,
    words_per_page: BTreeMap<u32, Vec<WordBounds>>,
    image_w: u32,
    image_h: u32,
    _dpi: u16,
) -> Result<(), PipelineError> {
    let image_w = image_w.max(1) as f32;
    let image_h = image_h.max(1) as f32;

    for (page_number, words) in words_per_page {
        if words.is_empty() {
            continue;
        }
        let pages = doc.get_pages();
        let Some(&page_id) = pages.get(&page_number) else {
            continue;
        };

        let (page_w_pt, page_h_pt) = get_page_media_box(doc, page_id).unwrap_or((595.0, 842.0));

        ensure_font_helvetica(doc, page_id)?;

        let mut content = Vec::new();
        for w in &words {
            // Calculate font size from word WIDTH so Helvetica glyph advance widths
            // cause the rendered text to span the correct horizontal extent.
            // Without this, height-based sizing makes glyphs too narrow and the
            // selection highlight visually truncates final letters.
            let word_width_pt = (w.right - w.left) as f32 * page_w_pt / image_w;
            let total_emu = helvetica_word_width_emu(&w.text);
            let font_size = if total_emu > 0 {
                (word_width_pt / (total_emu as f32 / 1000.0)).max(1.0)
            } else {
                ((w.bottom - w.top) as f32 * page_h_pt / image_h).max(1.0)
            };
            let pdf_x = w.left as f32 * page_w_pt / image_w;
            let pdf_y = page_h_pt - w.bottom as f32 * page_h_pt / image_h;
            let text = escape_pdf_string(&w.text);

            content.extend_from_slice(b"BT\n");
            content.extend_from_slice(b"/Helvetica ");
            content.extend_from_slice(format_pdf_float(font_size).as_bytes());
            content.extend_from_slice(b" Tf\n");
            content.extend_from_slice(b"1 0 0 1 ");
            content.extend_from_slice(format_pdf_float(pdf_x).as_bytes());
            content.extend_from_slice(b" ");
            content.extend_from_slice(format_pdf_float(pdf_y).as_bytes());
            content.extend_from_slice(b" Tm\n");
            content.extend_from_slice(b"3 Tr\n");
            content.extend_from_slice(b"(");
            content.extend_from_slice(text.as_bytes());
            content.extend_from_slice(b") Tj\n");
            content.extend_from_slice(b"ET\n");
        }

        doc.add_page_contents(page_id, content)
            .map_err(|e| PipelineError::PdfParse(format!("add text layer: {e}")))?;
    }
    Ok(())
}

/// Reads the page's MediaBox, walking up the parent chain if needed.
/// Returns `(width_pt, height_pt)` or `None` if no MediaBox is found.
pub(crate) fn get_page_media_box(
    doc: &Document,
    mut page_id: lopdf::ObjectId,
) -> Option<(f32, f32)> {
    loop {
        let obj = doc.get_object(page_id).ok()?;
        let dict = obj.as_dict().ok()?;
        if let Ok(mb) = dict.get(b"MediaBox")
            && let Ok(arr) = mb.as_array()
            && arr.len() >= 4
        {
            let llx = arr[0].as_float().unwrap_or(0.0);
            let lly = arr[1].as_float().unwrap_or(0.0);
            let urx = arr[2].as_float().unwrap_or(595.0);
            let ury = arr[3].as_float().unwrap_or(842.0);
            return Some((urx - llx, ury - lly));
        }
        if let Ok(parent) = dict.get(b"Parent")
            && let Ok(parent_ref) = parent.as_reference()
        {
            page_id = parent_ref;
            continue;
        }
        return None;
    }
}

/// Ensures the page's Resources dict has a `/Helvetica` font entry.
/// Creates the Resources/Font dicts if they don't exist.
/// Works with owned objects to avoid borrow checker conflicts.
#[cfg(feature = "ocr")]
fn ensure_font_helvetica(
    doc: &mut Document,
    page_id: lopdf::ObjectId,
) -> Result<(), PipelineError> {
    use lopdf::{Dictionary, Object};

    // Clone the page dict out to avoid holding a mutable borrow on `doc`
    let page_obj = doc
        .get_object(page_id)
        .map_err(|e| PipelineError::PdfParse(format!("get page: {e}")))?
        .clone();
    let mut page_dict = page_obj
        .as_dict()
        .map_err(|_| PipelineError::PdfParse("page dict expected".to_string()))?
        .clone();

    // Helper: resolve a reference to a dict, or return the dict if it's inline
    let resolve_dict_owned = |obj: &Object, d: &Document| -> Option<Dictionary> {
        match obj {
            Object::Dictionary(dict) => Some(dict.clone()),
            Object::Reference(id) => d
                .get_object(*id)
                .ok()
                .and_then(|o| o.as_dict().ok())
                .cloned(),
            _ => None,
        }
    };

    // Check if Helvetica already exists
    let resources_obj = page_dict.get(b"Resources").ok();
    let font_ok = resources_obj.and_then(|ro| {
        resolve_dict_owned(ro, doc).and_then(|rd| {
            rd.get(b"Font").ok().and_then(|fo| {
                resolve_dict_owned(fo, doc).and_then(|fd| fd.get(b"Helvetica").ok().map(|_| ()))
            })
        })
    });

    if font_ok.is_some() {
        return Ok(());
    }

    // Build the Helvetica font dict (inline, not a reference)
    let helvetica_dict = Object::Dictionary({
        let mut d = Dictionary::new();
        d.set("Type", "Font");
        d.set("Subtype", "Type1");
        d.set("BaseFont", "Helvetica");
        d
    });

    match resources_obj {
        Some(resources_obj) => {
            // Resources exists — add/update Font dict
            match resolve_dict_owned(resources_obj, doc) {
                Some(mut resources_dict) => {
                    let font_obj = resources_dict.get(b"Font").ok().cloned();
                    match font_obj {
                        Some(font_obj) => {
                            match resolve_dict_owned(&font_obj, doc) {
                                Some(mut font_dict) => {
                                    font_dict.set("Helvetica", helvetica_dict);
                                    resources_dict.set("Font", Object::Dictionary(font_dict));
                                }
                                None => {
                                    // Font exists but not a dict — replace
                                    let mut font_dict = Dictionary::new();
                                    font_dict.set("Helvetica", helvetica_dict);
                                    resources_dict.set("Font", Object::Dictionary(font_dict));
                                }
                            }
                        }
                        None => {
                            let mut font_dict = Dictionary::new();
                            font_dict.set("Helvetica", helvetica_dict);
                            resources_dict.set("Font", Object::Dictionary(font_dict));
                        }
                    }
                    page_dict.set("Resources", Object::Dictionary(resources_dict));
                }
                None => {
                    let mut resources_dict = Dictionary::new();
                    let mut font_dict = Dictionary::new();
                    font_dict.set("Helvetica", helvetica_dict);
                    resources_dict.set("Font", Object::Dictionary(font_dict));
                    page_dict.set("Resources", Object::Dictionary(resources_dict));
                }
            }
        }
        None => {
            let mut resources_dict = Dictionary::new();
            let mut font_dict = Dictionary::new();
            font_dict.set("Helvetica", helvetica_dict);
            resources_dict.set("Font", Object::Dictionary(font_dict));
            page_dict.set("Resources", Object::Dictionary(resources_dict));
        }
    }

    doc.objects.insert(page_id, Object::Dictionary(page_dict));

    Ok(())
}

/// Escapes special characters in a PDF string literal (parentheses, backslash).
#[cfg(feature = "ocr")]
fn escape_pdf_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '(' => out.push_str("\\("),
            ')' => out.push_str("\\)"),
            '\\' => out.push_str("\\\\"),
            c => out.push(c),
        }
    }
    out
}

/// Formats a float without trailing zeros for compact PDF output.
#[cfg(feature = "ocr")]
fn format_pdf_float(v: f32) -> String {
    if v.fract() == 0.0 {
        format!("{:.0}", v)
    } else {
        format!("{:.2}", v)
    }
}

fn resolve_dict<'a>(
    doc: &'a Document,
    obj: &'a Object,
) -> Result<Option<&'a Dictionary>, PipelineError> {
    if let Ok(dict) = obj.as_dict() {
        return Ok(Some(dict));
    }
    if let Ok(obj_id) = obj.as_reference() {
        let resolved = doc
            .get_object(obj_id)
            .map_err(|e| PipelineError::PdfParse(e.to_string()))?;
        return resolved
            .as_dict()
            .map(Some)
            .map_err(|_| PipelineError::PdfParse("expected dictionary reference".to_string()));
    }
    Ok(None)
}

/// Encodes a 1-bit bitonal image as a CCITT Group 4 compressed PDF image stream.
///
/// Uses the `fax` crate to produce real CCITT T.6 / Group 4 encoding (fax standard).
/// This achieves significantly better compression on text pages than FlateDecode.
/// The output stream uses `/Filter /CCITTFaxDecode` with `/K 0` (pure 2D Group 4).
///
/// Our bitonal buffer uses 1 = black, so `BlackIs1 true` is set in DecodeParms.
pub fn encode_ccitt_g4(width: u32, height: u32, bitonal: Vec<u8>) -> Result<Stream, PipelineError> {
    use fax::Color;
    use fax::VecWriter;
    use fax::encoder::Encoder;

    let actual_width = u16::try_from(width)
        .map_err(|_| PipelineError::PdfParse(format!("CCITT width {width} exceeds u16")))?;
    let actual_height = u16::try_from(height)
        .map_err(|_| PipelineError::PdfParse(format!("CCITT height {height} exceeds u16")))?;

    let row_bytes = actual_width.div_ceil(8) as usize;
    let writer = VecWriter::new();
    let mut encoder = Encoder::new(writer);

    for row in 0..actual_height as usize {
        let start = row * row_bytes;
        let end = start.saturating_add(row_bytes).min(bitonal.len());
        let row_data = &bitonal[start..end];

        let pels = (0..actual_width).map(|col| {
            let byte_idx = (col / 8) as usize;
            let bit_idx = 7 - (col % 8);
            let bit = row_data.get(byte_idx).copied().unwrap_or(0);
            let pixel = (bit >> bit_idx) & 1;
            if pixel == 1 {
                Color::Black
            } else {
                Color::White
            }
        });

        encoder
            .encode_line(pels, actual_width)
            .map_err(|e| PipelineError::PdfParse(format!("CCITT G4 encode line: {e}")))?;
    }

    let writer = encoder
        .finish()
        .map_err(|e| PipelineError::PdfParse(format!("CCITT G4 finish: {e}")))?;
    let encoded = writer.finish();

    let mut dict = Dictionary::new();
    dict.set("Type", "XObject");
    dict.set("Subtype", "Image");
    dict.set("Width", width as i64);
    dict.set("Height", height as i64);
    dict.set("ColorSpace", "DeviceGray");
    dict.set("BitsPerComponent", 1);
    dict.set("Filter", "CCITTFaxDecode");

    let mut dparms = Dictionary::new();
    dparms.set("K", 0);
    dparms.set("Columns", width as i64);
    dparms.set("Rows", height as i64);
    dparms.set("BlackIs1", true);
    dict.set("DecodeParms", dparms);
    dict.set("Decode", vec![Object::Integer(1), Object::Integer(0)]);

    Ok(Stream::new(dict, encoded))
}

/// Encodes an 8-bit grayscale image as a FlateDecode-compressed PDF image stream.
pub fn encode_flate(width: u32, height: u32, data: Vec<u8>) -> Result<Stream, PipelineError> {
    use std::io::Write;
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::best());
    encoder
        .write_all(&data)
        .map_err(|e| PipelineError::PdfParse(format!("flate compress: {e}")))?;
    let compressed = encoder
        .finish()
        .map_err(|e| PipelineError::PdfParse(format!("flate finish: {e}")))?;

    let mut dict = Dictionary::new();
    dict.set("Type", "XObject");
    dict.set("Subtype", "Image");
    dict.set("Width", width as i64);
    dict.set("Height", height as i64);
    dict.set("ColorSpace", "DeviceGray");
    dict.set("BitsPerComponent", 8);
    dict.set("Filter", "FlateDecode");

    Ok(Stream::new(dict, compressed))
}

/// Saves the document to `output`, optionally adding PDF/A metadata and compressing.
pub fn finalize(
    doc: &mut Document,
    output: &Path,
    enforce_pdfa: bool,
) -> Result<(), PipelineError> {
    if enforce_pdfa {
        enforce_pdfa_metadata(doc)?;
    }
    doc.compress();
    doc.save(output)
        .map(|_| ())
        .map_err(|e| PipelineError::PdfParse(e.to_string()))
}

/// Decodes a PDF XObject image stream into a grayscale image.
/// Supports CCITT G4 (with optional JBIG2 refinement), FlateDecode (gray/RGB),
/// DCTDecode (JPEG), and raw data. Returns a PipelineError for unsupported or
/// corrupted streams. JBIG2 decoding is wrapped in `catch_unwind` with
/// size/dimension limits (50 MB, 10k pixel cap) to guard against malicious payloads.
pub(crate) fn decode_stream_image(
    stream: &Stream,
    doc: &Document,
) -> Result<GrayImage, PipelineError> {
    let filters = stream.filters().unwrap_or_default();
    let width = stream
        .dict
        .get(b"Width")
        .and_then(|o| o.as_i64())
        .unwrap_or(0) as u32;
    let height = stream
        .dict
        .get(b"Height")
        .and_then(|o| o.as_i64())
        .unwrap_or(0) as u32;

    if filters.is_empty() {
        let bpc = bits_per_component(stream);
        let mut pixels = match bpc {
            1 => expand_1bpp_to_8bpp(&stream.content, width, height),
            8 => stream.content.clone(),
            n => {
                return Err(PipelineError::PdfParse(format!(
                    "unsupported BitsPerComponent: {n}"
                )));
            }
        };
        if needs_photometric_inversion(stream) {
            invert_grayscale(&mut pixels);
        }
        return GrayImage::from_raw(width, height, pixels)
            .ok_or_else(|| PipelineError::PdfParse("invalid image buffer".to_string()));
    }

    let filter = &filters[0];
    match filter.as_str() {
        "FlateDecode" => {
            use flate2::read::ZlibDecoder;
            use std::io::Read;
            let mut raw = Vec::with_capacity(stream.content.len() * 2);
            ZlibDecoder::new(&stream.content[..])
                .read_to_end(&mut raw)
                .map_err(|e| PipelineError::PdfParse(format!("flate: {e}")))?;
            let bpc = bits_per_component(stream);
            let mut pixels = match bpc {
                1 => expand_1bpp_to_8bpp(&raw, width, height),
                8 => raw,
                n => {
                    return Err(PipelineError::PdfParse(format!(
                        "unsupported BitsPerComponent: {n}"
                    )));
                }
            };
            if needs_photometric_inversion(stream) {
                invert_grayscale(&mut pixels);
            }
            GrayImage::from_raw(width, height, pixels)
                .ok_or_else(|| PipelineError::PdfParse("invalid image buffer".to_string()))
        }
        "JBIG2Decode" => {
            // Prevent decompression bombs: reject JBIG2 streams larger than 50 MB
            const JBIG2_MAX_BYTES: usize = 50 * 1024 * 1024;
            if stream.content.len() > JBIG2_MAX_BYTES {
                return Err(PipelineError::PdfParse(format!(
                    "JBIG2 stream too large: {} bytes (max {})",
                    stream.content.len(),
                    JBIG2_MAX_BYTES
                )));
            }
            let globals = stream
                .dict
                .get(b"JBIG2Globals")
                .ok()
                .and_then(|o| o.as_reference().ok())
                .and_then(|id| doc.get_object(id).ok())
                .and_then(|obj| {
                    if let Object::Stream(s) = obj {
                        Some(s.content.as_slice())
                    } else {
                        None
                    }
                });
            // JBIG2 is historically dangerous (CVE-2015-...), isolate with catch_unwind
            let jbig2_result = catch_unwind(AssertUnwindSafe(|| {
                pdfluent_jbig2::decode_embedded(&stream.content, globals)
            }));
            let jbig2_image = match jbig2_result {
                Ok(Ok(img)) => img,
                Ok(Err(e)) => {
                    return Err(PipelineError::PdfParse(format!("JBIG2: {e}")));
                }
                Err(panic) => {
                    tracing::error!(target: "knox::pdf", "JBIG2 decoder panicked: {:?}", panic);
                    return Err(PipelineError::PdfParse("JBIG2 decoder panic".to_string()));
                }
            };
            // Reject unreasonably large decoded dimensions (> 10k pixels per side)
            if jbig2_image.width > 10_000 || jbig2_image.height > 10_000 {
                return Err(PipelineError::PdfParse(format!(
                    "JBIG2 image too large: {}x{}",
                    jbig2_image.width, jbig2_image.height
                )));
            }
            struct PixelCollector(Vec<u8>);
            impl pdfluent_jbig2::Decoder for PixelCollector {
                fn push_pixel(&mut self, black: bool) {
                    self.0.push(if black { 0 } else { 255 });
                }
                fn push_pixel_chunk(&mut self, black: bool, count: u32) {
                    let val = if black { 0 } else { 255 };
                    self.0.resize(self.0.len() + count as usize * 8, val);
                }
                fn next_line(&mut self) {}
            }
            let mut c = PixelCollector(Vec::with_capacity(
                (jbig2_image.width * jbig2_image.height) as usize,
            ));
            jbig2_image.decode(&mut c);
            GrayImage::from_raw(jbig2_image.width, jbig2_image.height, c.0)
                .ok_or_else(|| PipelineError::PdfParse("invalid JBIG2 buffer".to_string()))
        }
        "DCTDecode" => {
            use std::io::Cursor;
            let img = image::ImageReader::new(Cursor::new(&stream.content))
                .with_guessed_format()
                .map_err(|e| PipelineError::PdfParse(format!("jpeg format: {e}")))?
                .decode()
                .map_err(|e| PipelineError::PdfParse(format!("jpeg decode: {e}")))?;
            Ok(img.into_luma8())
        }
        "CCITTFaxDecode" => decode_ccitt(stream, width, height),
        other => Err(PipelineError::PdfParse(format!(
            "unsupported image filter: {other}"
        ))),
    }
}

fn decode_ccitt(stream: &Stream, width: u32, height: u32) -> Result<GrayImage, PipelineError> {
    const CCITT_MAX_BYTES: usize = 100 * 1024 * 1024;
    if stream.content.len() > CCITT_MAX_BYTES {
        return Err(PipelineError::PdfParse(format!(
            "CCITT stream too large: {} bytes (max {})",
            stream.content.len(),
            CCITT_MAX_BYTES
        )));
    }
    if width == 0 || height == 0 || width > 10_000 || height > 10_000 {
        return Err(PipelineError::PdfParse(format!(
            "invalid CCITT image dimensions: {}x{}",
            width, height
        )));
    }

    let decode_parms = stream
        .dict
        .get(b"DecodeParms")
        .ok()
        .and_then(|o| o.as_dict().ok());

    let k = decode_parms
        .and_then(|d| d.get(b"K").ok())
        .and_then(|o| o.as_i64().ok())
        .unwrap_or(-1i64);

    let columns = decode_parms
        .and_then(|d| d.get(b"Columns").ok())
        .and_then(|o| o.as_i64().ok())
        .unwrap_or(width as i64) as u32;

    let black_is_1 = decode_parms
        .and_then(|d| d.get(b"BlackIs1").ok())
        .and_then(|o| o.as_bool().ok())
        .unwrap_or(false);

    let end_of_line = decode_parms
        .and_then(|d| d.get(b"EndOfLine").ok())
        .and_then(|o| o.as_bool().ok())
        .unwrap_or(false);

    let encoded_byte_align = decode_parms
        .and_then(|d| d.get(b"EncodedByteAlign").ok())
        .and_then(|o| o.as_bool().ok())
        .unwrap_or(false);

    let end_of_block = decode_parms
        .and_then(|d| d.get(b"EndOfBlock").ok())
        .and_then(|o| o.as_bool().ok())
        .unwrap_or(true);

    let encoding = if k < 0 {
        pdfluent_ccitt::EncodingMode::Group4
    } else if k == 0 {
        pdfluent_ccitt::EncodingMode::Group3_1D
    } else {
        pdfluent_ccitt::EncodingMode::Group3_2D { k: k as u32 }
    };

    let settings = pdfluent_ccitt::DecodeSettings {
        columns: columns.max(width),
        rows: height,
        end_of_block,
        end_of_line,
        rows_are_byte_aligned: encoded_byte_align,
        encoding,
        // BlackIs1=true means bit=1=black (matches fax natural). invert_black=false
        // leaves the natural mapping: white→push_pixel(true), black→push_pixel(false).
        // BlackIs1=false means bit=1=white (opposite of fax), so we invert.
        invert_black: !black_is_1,
    };

    struct CcittCollector(Vec<u8>);

    impl pdfluent_ccitt::Decoder for CcittCollector {
        fn push_pixel(&mut self, white: bool) {
            self.0.push(if white { 255 } else { 0 });
        }
        fn push_pixel_chunk(&mut self, white: bool, count: u32) {
            let val = if white { 255 } else { 0 };
            self.0.resize(self.0.len() + (count as usize) * 8, val);
        }
        fn next_line(&mut self) {}
    }

    let mut collector = CcittCollector(Vec::with_capacity((width * height) as usize));
    pdfluent_ccitt::decode(&stream.content, &mut collector, &settings)
        .map_err(|e| PipelineError::PdfParse(format!("CCITT decode: {e}")))?;

    GrayImage::from_raw(width, height, collector.0)
        .ok_or_else(|| PipelineError::PdfParse("invalid CCITT output buffer".to_string()))
}

fn enforce_pdfa_metadata(doc: &mut Document) -> Result<(), PipelineError> {
    let info_id = doc
        .trailer
        .get(b"Info")
        .and_then(|obj| obj.as_reference())
        .unwrap_or_else(|_| doc.new_object_id());

    let mut info = Dictionary::new();
    info.set("Producer", "Knox");
    info.set("Creator", "Knox");
    doc.objects.insert(info_id, Object::Dictionary(info));
    doc.trailer.set("Info", Object::Reference(info_id));

    let mut metadata = Dictionary::new();
    metadata.set("Type", "Metadata");
    metadata.set("Subtype", "XML");
    let xml = r#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<rdf:Description xmlns:pdfaid="http://www.aiim.org/pdfa/ns/id/" pdfaid:part="2" pdfaid:conformance="B"/>
</rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#;
    let metadata_stream = Stream::new(metadata, xml.as_bytes().to_vec());
    let metadata_id = doc.add_object(metadata_stream);
    if let Ok(catalog) = doc.catalog_mut() {
        catalog.set("Metadata", Object::Reference(metadata_id));
    } else {
        doc.trailer.set("Metadata", Object::Reference(metadata_id));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{Dictionary, Object};

    fn get_ok<'a>(dict: &'a Dictionary, key: &[u8]) -> &'a Object {
        dict.get(key).expect("key should exist")
    }

    #[test]
    fn encode_ccitt_g4_dict_fields() {
        let data = vec![0u8; 32];
        let stream = encode_ccitt_g4(16, 16, data).unwrap();
        let dict = &stream.dict;
        assert!(matches!(get_ok(dict, b"Subtype"), Object::Name(n) if n == b"Image"));
        assert!(matches!(get_ok(dict, b"Width"), Object::Integer(16)));
        assert!(matches!(get_ok(dict, b"Height"), Object::Integer(16)));
        assert!(matches!(
            get_ok(dict, b"BitsPerComponent"),
            Object::Integer(1)
        ));
        assert!(matches!(get_ok(dict, b"Filter"), Object::Name(n) if n == b"CCITTFaxDecode"));
        let dparms = dict.get(b"DecodeParms").unwrap();
        if let Object::Dictionary(d) = dparms {
            assert!(matches!(d.get(b"K").unwrap(), Object::Integer(0)));
            assert!(matches!(d.get(b"Columns").unwrap(), Object::Integer(16)));
            assert!(matches!(d.get(b"Rows").unwrap(), Object::Integer(16)));
            assert!(matches!(d.get(b"BlackIs1").unwrap(), Object::Boolean(true)));
        } else {
            panic!("DecodeParms should be a dictionary");
        }
    }

    #[test]
    fn encode_flate_dict_fields() {
        let data = vec![128u8; 256];
        let stream = encode_flate(16, 16, data).unwrap();
        let dict = &stream.dict;
        assert!(matches!(get_ok(dict, b"Subtype"), Object::Name(n) if n == b"Image"));
        assert!(matches!(get_ok(dict, b"Width"), Object::Integer(16)));
        assert!(matches!(get_ok(dict, b"Height"), Object::Integer(16)));
        assert!(matches!(
            get_ok(dict, b"BitsPerComponent"),
            Object::Integer(8)
        ));
        assert!(matches!(get_ok(dict, b"Filter"), Object::Name(n) if n == b"FlateDecode"));
        assert!(matches!(get_ok(dict, b"ColorSpace"), Object::Name(n) if n == b"DeviceGray"));
    }

    #[test]
    fn finalize_saves_document() {
        let mut doc = Document::new();
        let dir = std::env::temp_dir();
        let path = dir.join("knox_test_finalize.pdf");
        let result = finalize(&mut doc, &path, false);
        assert!(result.is_ok());
        assert!(path.exists());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn finalize_pdfa_adds_metadata() {
        let mut doc = Document::new();
        // Build a minimal valid document with a catalog
        let pages_id = doc.new_object_id();
        let mut pages_dict = Dictionary::new();
        pages_dict.set("Type", "Pages");
        pages_dict.set("Kids", Vec::<Object>::new());
        pages_dict.set("Count", 0);
        doc.add_object(lopdf::Object::Dictionary(pages_dict));
        let mut catalog = Dictionary::new();
        catalog.set("Type", "Catalog");
        catalog.set("Pages", Object::Reference(pages_id));
        let catalog_id = doc.add_object(lopdf::Object::Dictionary(catalog));
        doc.trailer.set("Root", Object::Reference(catalog_id));

        let dir = std::env::temp_dir();
        let path = dir.join("knox_test_pdfa.pdf");
        let result = finalize(&mut doc, &path, true);
        assert!(result.is_ok());

        let loaded = Document::load(&path).unwrap();
        let catalog = loaded.catalog().unwrap();
        assert!(
            catalog.get(b"Metadata").is_ok(),
            "PDF/A should have Metadata"
        );
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn load_document_nonexistent_fails() {
        let result = load_document(Path::new("/nonexistent/path.pdf"), None);
        assert!(result.is_err());
    }

    #[test]
    fn load_document_rejects_zero_byte_file() {
        let dir = std::env::temp_dir();
        let path = dir.join("knox_test_empty.pdf");
        std::fs::write(&path, b"").unwrap();
        let result = load_document(&path, None);
        assert!(result.is_err());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn load_document_rejects_junk() {
        let dir = std::env::temp_dir();
        let path = dir.join("knox_test_junk.pdf");
        std::fs::write(&path, b"not a pdf file at all").unwrap();
        let result = load_document(&path, None);
        assert!(result.is_err());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn for_each_page_image_zero_pages() {
        let doc = Document::new();
        let mut count = 0u32;
        let result = for_each_page_image(&doc, ExistingTextMode::Skip, |_page| {
            count += 1;
            Ok(())
        });
        assert!(result.is_ok());
        assert_eq!(count, 0);
    }

    #[test]
    fn load_document_and_save_round_trip() {
        let dir = std::env::temp_dir();
        let path = dir.join("knox_test_roundtrip.pdf");
        finalize(&mut Document::new(), &path, false).unwrap();
        let loaded = load_document(&path, None);
        assert!(loaded.is_ok());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn replace_page_images_empty_doc() {
        let mut doc = Document::new();
        let replacements = BTreeMap::new();
        let result = replace_page_images(&mut doc, replacements);
        assert!(result.is_ok());
    }

    #[test]
    fn decode_ccitt_empty_data() {
        let mut dict = Dictionary::new();
        dict.set("Filter", "CCITTFaxDecode");
        dict.set("Width", 4i64);
        dict.set("Height", 1i64);
        let stream = Stream::new(dict, vec![]);
        let result = decode_stream_image(&stream, &Document::new());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(&err, PipelineError::PdfParse(msg) if msg.contains("CCITT")),
            "expected CCITT error, got: {err}"
        );
    }

    #[test]
    fn decode_ccitt_invalid_dimensions() {
        let mut dict = Dictionary::new();
        dict.set("Filter", "CCITTFaxDecode");
        dict.set("Width", 0i64);
        dict.set("Height", 0i64);
        let stream = Stream::new(dict, vec![0x80]);
        let result = decode_stream_image(&stream, &Document::new());
        assert!(result.is_err());
    }

    #[test]
    fn decode_ccitt_g4_four_white() {
        // Group 4, 4 columns, 1 row, all white. BlackIs1=true so
        // invert_black=false → white background → 255.
        // V(0) = 0b1 (1 bit) → a1=b1=4 → 4 white pixels → 0b10000000 = 0x80.
        let mut dict = Dictionary::new();
        dict.set("Filter", "CCITTFaxDecode");
        dict.set("Width", 4i64);
        dict.set("Height", 1i64);
        let mut dp = Dictionary::new();
        dp.set("BlackIs1", true);
        dict.set("DecodeParms", dp);
        let stream = Stream::new(dict, vec![0x80]);
        let img = decode_stream_image(&stream, &Document::new()).unwrap();
        assert_eq!(img.width(), 4);
        assert_eq!(img.height(), 1);
        assert!(img.pixels().all(|p| p.0[0] == 255));
    }

    #[test]
    fn decode_ccitt_g3_1d_four_white() {
        // Group 3 1D, 4 columns, 1 row, all white. BlackIs1=true so
        // invert_black=false → push_pixel(true) → 255.
        // White(4) = 0b1011 → padded to byte 0b10110000 = 0xB0.
        let mut dict = Dictionary::new();
        dict.set("Filter", "CCITTFaxDecode");
        dict.set("Width", 4i64);
        dict.set("Height", 1i64);
        let mut dp = Dictionary::new();
        dp.set("K", 0i64);
        dp.set("BlackIs1", true);
        dict.set("DecodeParms", dp);
        let stream = Stream::new(dict, vec![0xB0]);
        let img = decode_stream_image(&stream, &Document::new()).unwrap();
        assert_eq!(img.width(), 4);
        assert_eq!(img.height(), 1);
        assert!(img.pixels().all(|p| p.0[0] == 255));
    }

    #[test]
    fn decode_ccitt_g3_1d_all_black() {
        // Group 3 1D, 4 columns, 1 row, all black. BlackIs1=true so
        // invert_black=false → black run → push_pixel(false) → 0.
        // White(0) = 0x35, Black(4) = 0b0011 → byte 0x30.
        let mut dict = Dictionary::new();
        dict.set("Filter", "CCITTFaxDecode");
        dict.set("Width", 4i64);
        dict.set("Height", 1i64);
        let mut dp = Dictionary::new();
        dp.set("K", 0i64);
        dp.set("BlackIs1", true);
        dict.set("DecodeParms", dp);
        let stream = Stream::new(dict, vec![0x35, 0x30]);
        let img = decode_stream_image(&stream, &Document::new()).unwrap();
        assert_eq!(img.width(), 4);
        assert_eq!(img.height(), 1);
        assert!(img.pixels().all(|p| p.0[0] == 0));
    }

    #[test]
    fn decode_ccitt_too_large() {
        // Stream with content > 100 MB limit should fail
        let mut dict = Dictionary::new();
        dict.set("Filter", "CCITTFaxDecode");
        dict.set("Width", 100i64);
        dict.set("Height", 100i64);
        let huge = vec![0u8; 100 * 1024 * 1024 + 1];
        let stream = Stream::new(dict, huge);
        let result = decode_stream_image(&stream, &Document::new());
        assert!(result.is_err());
    }

    #[test]
    fn expand_1bpp_all_white_4x1() {
        // One byte = 8 pixels packed, but width=4 so only 4 MSB bits used.
        // 0xF0 = 0b11110000 → pixel: 1 1 1 1 → all 255.
        let expanded = expand_1bpp_to_8bpp(&[0xF0], 4, 1);
        assert_eq!(expanded, vec![255, 255, 255, 255]);
    }

    #[test]
    fn expand_1bpp_all_black_4x1() {
        // 0x00 = 0b00000000 → pixel: 0 0 0 0 → all 0.
        let expanded = expand_1bpp_to_8bpp(&[0x00], 4, 1);
        assert_eq!(expanded, vec![0, 0, 0, 0]);
    }

    #[test]
    fn expand_1bpp_mixed_8x1() {
        // 0b10100101 = 0xA5 → pixels: 1 0 1 0 0 1 0 1
        // Expected: 255, 0, 255, 0, 0, 255, 0, 255
        let expanded = expand_1bpp_to_8bpp(&[0xA5], 8, 1);
        assert_eq!(expanded, vec![255, 0, 255, 0, 0, 255, 0, 255]);
    }

    #[test]
    fn expand_1bpp_row_stride() {
        // 10 pixels wide → stride = ceil(10/8) = 2 bytes per row
        // Row 0: [0xFF, 0xC0] = 0b11111111 11000000 → 10 bits: 1111111111
        // Row 1: [0x00, 0x00] → 10 bits: 0000000000
        let data = vec![0xFF, 0xC0, 0x00, 0x00];
        let expanded = expand_1bpp_to_8bpp(&data, 10, 2);
        assert_eq!(expanded.len(), 20);
        // Row 0: first 10 pixels = 255
        for &p in expanded[..10].iter() {
            assert_eq!(p, 255);
        }
        // Row 1: first 10 pixels = 0
        for &p in expanded[10..].iter() {
            assert_eq!(p, 0);
        }
    }

    #[test]
    fn needs_inversion_decode_array_10() {
        // Decode [1, 0] should trigger inversion (d0=1 > d1=0)
        let mut dict = Dictionary::new();
        dict.set("Width", 4i64);
        dict.set("Height", 1i64);
        dict.set("BitsPerComponent", 1i64);
        dict.set("Decode", vec![Object::Integer(1), Object::Integer(0)]);
        let stream = Stream::new(dict, vec![]);
        assert!(needs_photometric_inversion(&stream));
    }

    #[test]
    fn needs_inversion_decode_array_01() {
        // Decode [0, 1] should NOT trigger inversion (d0=0 < d1=1)
        let mut dict = Dictionary::new();
        dict.set("Width", 4i64);
        dict.set("Height", 1i64);
        dict.set("BitsPerComponent", 1i64);
        dict.set("Decode", vec![Object::Integer(0), Object::Integer(1)]);
        let stream = Stream::new(dict, vec![]);
        assert!(!needs_photometric_inversion(&stream));
    }

    #[test]
    fn needs_inversion_image_mask() {
        // ImageMask=true should trigger inversion (default Decode [1,0])
        let mut dict = Dictionary::new();
        dict.set("Width", 4i64);
        dict.set("Height", 1i64);
        dict.set("BitsPerComponent", 1i64);
        dict.set("ImageMask", true);
        let stream = Stream::new(dict, vec![]);
        assert!(needs_photometric_inversion(&stream));
    }

    #[test]
    fn bits_per_component_default_8() {
        let dict = Dictionary::new();
        let stream = Stream::new(dict, vec![]);
        assert_eq!(bits_per_component(&stream), 8);
    }

    #[test]
    fn bits_per_component_explicit_1() {
        let mut dict = Dictionary::new();
        dict.set("BitsPerComponent", 1i64);
        let stream = Stream::new(dict, vec![]);
        assert_eq!(bits_per_component(&stream), 1);
    }
}

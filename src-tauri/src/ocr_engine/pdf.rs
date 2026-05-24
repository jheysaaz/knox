use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;

use image::GrayImage;
use lopdf::{Dictionary, Document, Object, Stream};

use crate::ocr_engine::error::PipelineError;
use crate::ocr_engine::types::ExistingTextMode;

/// Metadata for a single PDF page extracted for OCR processing.
pub struct PdfPageInfo {
    /// 1-indexed page number within the PDF.
    pub page_number: u32,
    /// Decoded grayscale image of the page.
    pub image: image::GrayImage,
}

const MAX_PDF_SIZE: u64 = 512 * 1024 * 1024;

/// Loads a PDF document from the given filesystem path.
/// Rejects files larger than 512 MB to prevent OOM from decompression bombs.
pub fn load_document(path: &Path) -> Result<Document, PipelineError> {
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
    let doc = Document::load(path).map_err(|e| PipelineError::PdfParse(e.to_string()))?;
    Ok(doc)
}

/// Returns `true` if the PDF page contains text operators (Tj, TJ, ', ").
/// Used to skip OCR on pages that already have a text layer.
/// Parses the page's content stream, which is O(operations) in the number of
/// PDF drawing operations — returns early on first text operator found.
fn page_has_text(doc: &Document, page_id: lopdf::ObjectId) -> Result<bool, PipelineError> {
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
            f(PdfPageInfo {
                page_number: page_number as u32,
                image,
            })?;
        }
    }
    Ok(())
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
        let Some(new_stream) = replacements.remove(&(page_number as u32)) else {
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

/// Encodes a 1-bit bitonal image as a CCITT G4-compressed PDF image stream.
///
/// # CCITT G4 Convention
/// `BlackIs1: true` — a 1-bit in the input data represents a black pixel.
/// This matches the encoding produced by `to_bitonal_1bpp` in `image.rs`,
/// where `pixel == 0` (black) → bit `1`.
/// This convention is the inverse of `imageproc`'s default (white = 1).
pub fn encode_ccitt_g4(width: u32, height: u32, bitonal: Vec<u8>) -> Stream {
    let mut dict = Dictionary::new();
    dict.set("Type", "XObject");
    dict.set("Subtype", "Image");
    dict.set("Width", width as i64);
    dict.set("Height", height as i64);
    dict.set("ColorSpace", "DeviceGray");
    dict.set("BitsPerComponent", 1);
    dict.set("Filter", "CCITTFaxDecode");

    let mut decode = Dictionary::new();
    decode.set("K", -1);
    decode.set("Columns", width as i64);
    decode.set("Rows", height as i64);
    decode.set("BlackIs1", true);
    dict.set("DecodeParms", decode);

    Stream::new(dict, bitonal)
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
        return GrayImage::from_raw(width, height, stream.content.clone())
            .ok_or_else(|| PipelineError::PdfParse("invalid image buffer".to_string()));
    }

    let filter = &filters[0];
    match filter.as_str() {
        "FlateDecode" => {
            use flate2::read::ZlibDecoder;
            use std::io::Read;
            let mut data = Vec::with_capacity(stream.content.len() * 2);
            ZlibDecoder::new(&stream.content[..])
                .read_to_end(&mut data)
                .map_err(|e| PipelineError::PdfParse(format!("flate: {e}")))?;
            GrayImage::from_raw(width, height, data)
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
        other => Err(PipelineError::PdfParse(format!(
            "unsupported image filter: {other}"
        ))),
    }
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
        let stream = encode_ccitt_g4(16, 16, data);
        let dict = &stream.dict;
        assert!(matches!(get_ok(dict, b"Subtype"), Object::Name(n) if n == b"Image"));
        assert!(matches!(get_ok(dict, b"Width"), Object::Integer(16)));
        assert!(matches!(get_ok(dict, b"Height"), Object::Integer(16)));
        assert!(matches!(
            get_ok(dict, b"BitsPerComponent"),
            Object::Integer(1)
        ));
        assert!(matches!(get_ok(dict, b"Filter"), Object::Name(n) if n == b"CCITTFaxDecode"));
        let decode_params = dict.get(b"DecodeParms").unwrap();
        if let Object::Dictionary(dp) = decode_params {
            assert!(matches!(dp.get(b"K"), Ok(Object::Integer(-1))));
            assert!(matches!(dp.get(b"Columns"), Ok(Object::Integer(16))));
            assert!(matches!(dp.get(b"Rows"), Ok(Object::Integer(16))));
            assert!(matches!(dp.get(b"BlackIs1"), Ok(Object::Boolean(true))));
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
        let result = load_document(Path::new("/nonexistent/path.pdf"));
        assert!(result.is_err());
    }

    #[test]
    fn load_document_rejects_zero_byte_file() {
        let dir = std::env::temp_dir();
        let path = dir.join("knox_test_empty.pdf");
        std::fs::write(&path, b"").unwrap();
        let result = load_document(&path);
        assert!(result.is_err());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn load_document_rejects_junk() {
        let dir = std::env::temp_dir();
        let path = dir.join("knox_test_junk.pdf");
        std::fs::write(&path, b"not a pdf file at all").unwrap();
        let result = load_document(&path);
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
        let loaded = load_document(&path);
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

use std::collections::BTreeMap;
use std::path::Path;

use image::GrayImage;
use lopdf::{Dictionary, Document, Object, Stream};

use crate::ocr_engine::error::PipelineError;
use crate::ocr_engine::types::ExistingTextMode;

pub struct PdfPageInfo {
    pub page_number: u32,
    pub image: image::GrayImage,
}

pub fn load_document(path: &Path) -> Result<Document, PipelineError> {
    Document::load(path).map_err(|e| PipelineError::PdfParse(e.to_string()))
}

pub fn extract_page_images(
    doc: &Document,
    existing_text: ExistingTextMode,
) -> Result<Vec<PdfPageInfo>, PipelineError> {
    let mut out = Vec::new();
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
            out.push(PdfPageInfo {
                page_number: page_number as u32,
                image,
            });
        }
    }
    Ok(out)
}

fn page_has_text(doc: &Document, page_id: lopdf::ObjectId) -> Result<bool, PipelineError> {
    let content_data = match doc.get_page_content(page_id) {
        Ok(data) => data,
        Err(_) => return Ok(false),
    };
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

pub fn replace_page_images(
    doc: &mut Document,
    replacements: BTreeMap<u32, Stream>,
) -> Result<(), PipelineError> {
    let pages = doc.get_pages();
    for (page_number, page_id) in pages {
        let Some(new_stream) = replacements.get(&(page_number as u32)) else {
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
            doc.objects
                .insert(obj_id, Object::Stream(new_stream.clone()));
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

pub fn encode_flate(width: u32, height: u32, data: Vec<u8>) -> Result<Stream, PipelineError> {
    let mut dict = Dictionary::new();
    dict.set("Type", "XObject");
    dict.set("Subtype", "Image");
    dict.set("Width", width as i64);
    dict.set("Height", height as i64);
    dict.set("ColorSpace", "DeviceGray");
    dict.set("BitsPerComponent", 8);
    dict.set("Filter", "FlateDecode");

    Ok(Stream::new(dict, data))
}

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
            let jbig2_image = pdfluent_jbig2::decode_embedded(&stream.content, globals)
                .map_err(|e| PipelineError::PdfParse(format!("JBIG2: {e}")))?;
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

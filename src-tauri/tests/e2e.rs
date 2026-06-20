#![cfg(all(feature = "integration", feature = "ocr"))]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use image::{GrayImage, Luma};
use knox_lib::ocr_engine::error::PipelineError;
use knox_lib::ocr_engine::image::preprocess;
use knox_lib::ocr_engine::ocr::{TessApi, WordBounds};
use knox_lib::ocr_engine::pdf::{
    add_text_layers, encode_ccitt_g4, extract_lopdf_page, finalize, load_document,
    replace_page_images,
};
use knox_lib::ocr_engine::render::PdfiumEngine;
use knox_lib::ocr_engine::types::{
    BinarizationMode, CompressionMode, DeskewMode, ExistingTextMode, OcrSettings, PageSegMode,
};
use lopdf::{Dictionary, Document, Object, Stream};

/// Renders `text` onto a white 300x100 GrayImage using the font at `font_path`.
fn render_text_to_image(text: &str, font_path: &Path) -> GrayImage {
    use ab_glyph::{FontRef, PxScale};

    let font_data = std::fs::read(font_path).expect("failed to read font file");
    let font = FontRef::try_from_slice(&font_data).expect("invalid font");

    let mut img = GrayImage::from_pixel(300, 100, Luma([255u8]));
    let scale = PxScale::from(28.0);

    let _ = imageproc::drawing::draw_text(&mut img, Luma([0u8]), 20, 30, scale, &font, text);
    img
}

/// Builds a valid multi-page PDF at `path` from a list of GrayImages.
fn create_synthetic_pdf(path: &Path, pages: &[GrayImage]) {
    let mut doc = Document::new();
    let mut page_ids = Vec::new();

    for (_i, img) in pages.iter().enumerate() {
        let (w, h) = img.dimensions();

        // Raw grayscale bytes → FlateDecode stream
        let raw = img.to_vec();
        use flate2::Compression;
        use flate2::write::ZlibEncoder;
        let compressed = {
            let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
            std::io::Write::write_all(&mut e, &raw).unwrap();
            e.finish().unwrap()
        };

        let image_stream = Stream::new(
            Dictionary::from_iter([
                ("Type", Object::Name(b"XObject".to_vec())),
                ("Subtype", Object::Name(b"Image".to_vec())),
                ("Width", Object::Integer(w as i64)),
                ("Height", Object::Integer(h as i64)),
                ("ColorSpace", Object::Name(b"DeviceGray".to_vec())),
                ("BitsPerComponent", Object::Integer(8)),
                ("Filter", Object::Name(b"FlateDecode".to_vec())),
            ]),
            compressed,
        );
        let image_id = doc.add_object(Object::Stream(image_stream));
        let image_ref = Object::Reference(image_id);

        // Content stream: place image at (0,0) scaled to fit at 72 DPI
        let content_body = format!("q {} 0 0 {} 0 0 cm /Im{} Do Q", w, h, image_id.0);
        let content_stream = Stream::new(Dictionary::new(), content_body.into_bytes());
        let content_id = doc.add_object(Object::Stream(content_stream));

        // Resources dictionary referencing the image XObject
        let xobject_dict = Dictionary::from_iter([(format!("Im{}", image_id.0), image_ref)]);
        let resources_dict = Dictionary::from_iter([("XObject", Object::Dictionary(xobject_dict))]);

        // Page dictionary
        let page_dict = Dictionary::from_iter([
            ("Type", Object::Name(b"Page".to_vec())),
            ("MediaBox", {
                let margin = 72u32;
                Object::Array(vec![
                    Object::Integer(0),
                    Object::Integer(0),
                    Object::Integer((w + margin * 2) as i64),
                    Object::Integer((h + margin * 2) as i64),
                ])
            }),
            ("Contents", Object::Reference(content_id)),
            ("Resources", Object::Dictionary(resources_dict)),
        ]);
        let page_id = doc.add_object(Object::Dictionary(page_dict));
        page_ids.push(page_id);
    }

    // Build page tree
    let pages_id = doc.new_object_id();
    let mut pages_dict = Dictionary::new();
    pages_dict.set("Type", Object::Name(b"Pages".to_vec()));
    pages_dict.set(
        "Kids",
        Object::Array(page_ids.iter().map(|&id| Object::Reference(id)).collect()),
    );
    pages_dict.set("Count", Object::Integer(page_ids.len() as i64));
    doc.objects.insert(pages_id, Object::Dictionary(pages_dict));

    // Set each page's Parent
    for &page_id in &page_ids {
        if let Some(Object::Dictionary(d)) = doc.objects.get_mut(&page_id) {
            d.set("Parent", Object::Reference(pages_id));
        }
    }

    // Catalog
    let mut catalog = Dictionary::new();
    catalog.set("Type", Object::Name(b"Catalog".to_vec()));
    catalog.set("Pages", Object::Reference(pages_id));
    let catalog_id = doc.add_object(Object::Dictionary(catalog));
    doc.trailer.set("Root", Object::Reference(catalog_id));

    doc.save(path).expect("failed to save synthetic PDF");
}

fn find_system_font() -> Option<PathBuf> {
    let candidates = [
        "/System/Library/Fonts/Helvetica.ttc",
        "/System/Library/Fonts/Supplemental/Arial.ttf",
        "/System/Library/Fonts/STHeiti Light.ttc",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        "/usr/share/fonts/TTF/DejaVuSans.ttf",
    ];
    for c in &candidates {
        let p = Path::new(c);
        if p.exists() {
            return Some(p.to_path_buf());
        }
    }
    None
}

/// Returns the path to a sample file in the project's `samples/` directory.
/// Resolution is relative to `CARGO_MANIFEST_DIR` (which points to `src-tauri/`
/// for integration tests).
fn sample_path(name: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("CARGO_MANIFEST_DIR has no parent (expected src-tauri/)");
    root.join("samples").join(name)
}

/// Asserts that `path` is a valid PDF with a catalog, page tree, and the
/// expected number of pages.
fn validate_output_pdf(path: &Path, expected_pages: u32) {
    assert!(
        path.exists(),
        "output PDF was not created: {}",
        path.display()
    );
    let meta = std::fs::metadata(path).expect("failed to stat output");
    assert!(meta.len() > 100, "output PDF too small ({})", meta.len());

    let doc = Document::load(path).expect("output is not a valid PDF");
    let catalog = doc.catalog().expect("output PDF has no catalog");
    assert!(catalog.get(b"Pages").is_ok(), "output PDF has no page tree");
    assert_eq!(
        doc.get_pages().len() as u32,
        expected_pages,
        "output PDF page count mismatch"
    );
}

/// Renders a PDF page to an image using pdfium (if `pdfium` is `Some` and the
/// library path is non-empty), falling back to lopdf embedded-image extraction.
fn render_or_extract_page(
    pdfium: Option<&PdfiumEngine>,
    pdf_path: &Path,
    doc: &Document,
    page_number: u32,
    dpi: u16,
) -> Result<Option<GrayImage>, PipelineError> {
    if let Some(pdfium) = pdfium {
        let result = pdfium.render_page(pdf_path, page_number - 1, dpi, None)?;
        if result.is_some() {
            return Ok(result);
        }
    }
    extract_lopdf_page(doc, page_number, ExistingTextMode::Rasterize)
}

/// Loads a sample PDF, processes every page through the full OCR pipeline
/// (render → preprocess → Tesseract → text layers → replace images → save),
/// then validates the output PDF structure.
fn run_sample_pipeline(sample_name: &str, tessdata: &str) {
    let src = sample_path(sample_name);
    assert!(src.exists(), "sample file not found: {}", src.display());

    let tmp = std::env::temp_dir();
    let input = tmp.join(format!("knox_e2e_input_{sample_name}"));
    let output = tmp.join(format!("knox_e2e_output_{sample_name}"));
    std::fs::copy(&src, &input).expect("failed to copy sample to temp");

    let pdfium_path = std::env::var("PDFIUM_LIB_PATH").ok();
    let pdfium = pdfium_path.as_ref().map(|p| PdfiumEngine::new(p.as_str()));
    let pdfium_ref = pdfium.as_ref();

    let mut doc = load_document(&input, None).expect("failed to load sample PDF");
    let total_pages = doc.get_pages().len() as u32;

    let settings = OcrSettings {
        binarization: BinarizationMode::Otsu,
        fixed_threshold: 128,
        deskew_mode: DeskewMode::Radon,
        denoise_level: 2,
        existing_text: ExistingTextMode::Skip,
        psm: PageSegMode::Auto,
        compression: CompressionMode::Ccitt,
        resolution_dpi: 300,
        archive_enforcement: false,
        continue_on_error: false,
        password: None,
    };

    let tess = TessApi::new(tessdata, "eng").expect("failed to init Tesseract");
    let mut ocr_results: BTreeMap<u32, Vec<WordBounds>> = BTreeMap::new();
    let mut replacements: BTreeMap<u32, lopdf::Stream> = BTreeMap::new();
    let mut processed_any = false;

    for (page_number, _page_id) in &doc.get_pages() {
        let image_opt = render_or_extract_page(
            pdfium_ref,
            &input,
            &doc,
            *page_number,
            settings.resolution_dpi,
        )
        .expect("page extraction failed");

        let Some(image) = image_opt else {
            eprintln!("WARN: page {page_number} in {sample_name} has no extractable image");
            continue;
        };
        processed_any = true;

        let processed = preprocess(&image, &settings, false).expect("preprocessing failed");

        tess.set_image_bytes(
            &processed.ocr_image.to_vec(),
            processed.ocr_image.width() as i32,
            processed.ocr_image.height() as i32,
            1,
            processed.ocr_image.width() as i32,
        )
        .expect("failed to set image bytes for OCR");

        let text = tess.get_text().expect("OCR failed");
        let words = tess.get_words().expect("failed to get word bounds");

        eprintln!("Page {page_number} OCR text ({sample_name}): {text:?}");

        if text.trim().is_empty() {
            eprintln!("WARN: page {page_number} produced no text");
        }

        ocr_results.insert(*page_number, words);

        if let Some(ref bitonal) = processed.bitonal {
            let stream = encode_ccitt_g4(
                processed.ocr_image.width(),
                processed.ocr_image.height(),
                bitonal.data.clone(),
            )
            .expect("CCITT G4 encoding failed");
            replacements.insert(*page_number, stream);
        }
    }

    assert!(
        processed_any,
        "{sample_name}: no pages had extractable images"
    );

    if !replacements.is_empty() {
        replace_page_images(&mut doc, replacements).expect("failed to replace page images");
    }

    add_text_layers(
        &mut doc,
        ocr_results,
        settings.resolution_dpi as u32,
        settings.resolution_dpi as u32,
        settings.resolution_dpi,
    )
    .expect("failed to add text layers");

    finalize(&mut doc, &output, settings.archive_enforcement).expect("failed to finalize PDF");

    validate_output_pdf(&output, total_pages);

    let _ = std::fs::remove_file(&input);
    let _ = std::fs::remove_file(&output);

    eprintln!("PASS: sample '{sample_name}' processed successfully");
}

#[test]
fn e2e_with_poster_pdf() {
    let tessdata = match std::env::var("TESSDATA_PREFIX") {
        Ok(val) => val,
        Err(_) => {
            eprintln!("SKIP: TESSDATA_PREFIX not set");
            return;
        }
    };
    run_sample_pipeline("poster.pdf", &tessdata);
}

#[test]
fn e2e_with_skew_pdf() {
    let tessdata = match std::env::var("TESSDATA_PREFIX") {
        Ok(val) => val,
        Err(_) => {
            eprintln!("SKIP: TESSDATA_PREFIX not set");
            return;
        }
    };
    run_sample_pipeline("skew.pdf", &tessdata);
}

#[test]
fn e2e_ocr_pipeline_round_trip() {
    let tessdata = match std::env::var("TESSDATA_PREFIX") {
        Ok(val) => val,
        Err(_) => {
            eprintln!("SKIP: TESSDATA_PREFIX not set");
            return;
        }
    };

    let font_path = match find_system_font() {
        Some(p) => p,
        None => {
            eprintln!("SKIP: no system font found for rendering test text");
            return;
        }
    };

    // ── Step 1: Create synthetic 2-page PDF ──
    let page1 = render_text_to_image("Hello", &font_path);
    let page2 = render_text_to_image("World", &font_path);

    let tmp = std::env::temp_dir();
    let input_path = tmp.join("knox_e2e_input.pdf");
    let output_path = tmp.join("knox_e2e_output.pdf");

    create_synthetic_pdf(&input_path, &[page1, page2]);

    // ── Step 2: Load the document ──
    let mut doc = load_document(&input_path, None).expect("failed to load synthetic PDF");

    // ── Step 3: Extract and process each page ──
    let settings = OcrSettings {
        binarization: BinarizationMode::Otsu,
        fixed_threshold: 128,
        deskew_mode: DeskewMode::Radon,
        denoise_level: 2,
        existing_text: ExistingTextMode::Skip,
        psm: PageSegMode::Auto,
        compression: CompressionMode::Ccitt,
        resolution_dpi: 300,
        archive_enforcement: false,
        continue_on_error: false,
        password: None,
    };

    let mut ocr_results: BTreeMap<u32, Vec<knox_lib::ocr_engine::ocr::WordBounds>> =
        BTreeMap::new();
    let tess = TessApi::new(&tessdata, "eng").expect("failed to init Tesseract");

    let pages = doc.get_pages();
    let original_page_count = pages.len();
    assert!(original_page_count >= 2, "expected at least 2 pages");

    for (page_number, _page_id) in &pages {
        // Extract embedded image from page
        let image_opt = extract_lopdf_page(&doc, *page_number, ExistingTextMode::Skip)
            .expect("failed to extract page image");
        let Some(image) = image_opt else {
            eprintln!("SKIP: page {page_number} has no embedded image");
            continue;
        };

        // Preprocess
        let processed = preprocess(&image, &settings, false).expect("preprocessing failed");

        // OCR
        tess.set_image_bytes(
            &processed.ocr_image.to_vec(),
            processed.ocr_image.width() as i32,
            processed.ocr_image.height() as i32,
            1,
            processed.ocr_image.width() as i32,
        )
        .expect("failed to set image bytes for OCR");

        let text = tess.get_text().expect("OCR failed");
        let words = tess.get_words().expect("failed to get word bounds");

        eprintln!("Page {page_number} OCR text: {text:?}");

        if words.is_empty() {
            eprintln!("WARN: page {page_number} produced no words");
        }

        ocr_results.insert(*page_number, words);

        // Encode bitonal image as CCITT G4
        if let Some(ref bitonal) = processed.bitonal {
            let stream = encode_ccitt_g4(
                processed.ocr_image.width(),
                processed.ocr_image.height(),
                bitonal.data.clone(),
            )
            .expect("CCITT G4 encoding failed");

            // Replace image in the document
            let mut replacements = BTreeMap::new();
            replacements.insert(*page_number, stream);
            replace_page_images(&mut doc, replacements).expect("failed to replace page images");
        }
    }

    // ── Step 4: Add text layers and save ──
    add_text_layers(
        &mut doc,
        ocr_results,
        settings.resolution_dpi as u32,
        settings.resolution_dpi as u32,
        settings.resolution_dpi,
    )
    .expect("failed to add text layers");

    finalize(&mut doc, &output_path, settings.archive_enforcement).expect("failed to finalize PDF");

    // ── Step 5: Verify output ──
    assert!(output_path.exists(), "output PDF was not created");
    let output_meta = std::fs::metadata(&output_path).expect("failed to stat output");
    assert!(
        output_meta.len() > 100,
        "output PDF too small ({})",
        output_meta.len()
    );

    // Verify output is a valid PDF with a catalog
    let output_doc = Document::load(&output_path).expect("output is not a valid PDF");
    let catalog = output_doc.catalog().expect("output PDF has no catalog");
    assert!(catalog.get(b"Pages").is_ok(), "output PDF has no page tree");

    // Cleanup
    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&output_path);

    eprintln!("PASS: e2e OCR pipeline round-trip completed successfully");
}

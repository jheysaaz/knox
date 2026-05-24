use std::path::Path;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use image::GrayImage;
use pdfium_render::prelude::*;

use crate::ocr_engine::error::PipelineError;

const PDFIUM_RENDER_TIMEOUT: Duration = Duration::from_secs(30);

/// Engine for full-page PDF rasterization via the pdfium library.
/// Does NOT try to load the dylib at construction — binding happens per
/// render call on a dedicated thread, so a corrupted/incompatible library
/// can't freeze the app at startup.
pub struct PdfiumEngine {
    lib_path: String,
}

impl PdfiumEngine {
    pub fn new(lib_path: &str) -> Self {
        tracing::debug!(
            target: "knox::render",
            path = lib_path,
            empty = lib_path.is_empty(),
            "PdfiumEngine created (lazy binding)"
        );
        Self {
            lib_path: lib_path.to_string(),
        }
    }

    /// Renders a single PDF page on a separate thread with a 30-second timeout.
    pub fn render_page(
        &self,
        pdf_path: &Path,
        page_index: u32,
        dpi: u16,
    ) -> Result<Option<GrayImage>, PipelineError> {
        if self.lib_path.is_empty() {
            return Ok(None);
        }
        let lib_path = self.lib_path.clone();
        let pdf_path = pdf_path.to_path_buf();
        let (tx, rx) = mpsc::channel();

        std::thread::spawn(move || {
            let result = render_pdfium_page(&lib_path, &pdf_path, page_index, dpi);
            let _ = tx.send(result);
        });

        match rx.recv_timeout(PDFIUM_RENDER_TIMEOUT) {
            Ok(result) => result,
            Err(_) => Err(PipelineError::Pdfium(
                "pdfium render timed out after 30s".to_string(),
            )),
        }
    }
}

fn render_pdfium_page(
    lib_path: &str,
    pdf_path: &Path,
    page_index: u32,
    dpi: u16,
) -> Result<Option<GrayImage>, PipelineError> {
    let t0 = Instant::now();
    tracing::debug!(target: "knox::render", path = %pdf_path.display(), page = page_index, "render_pdfium_page: bind_to_library");
    let bindings = Pdfium::bind_to_library(lib_path)
        .map_err(|e| PipelineError::Pdfium(format!("bind: {e}")))?;
    let pdfium = Pdfium::new(bindings);
    tracing::debug!(target: "knox::render", elapsed = t0.elapsed().as_millis(), "render_pdfium_page: bind done, load pdf");
    let t1 = Instant::now();
    let document = pdfium
        .load_pdf_from_file(pdf_path, None)
        .map_err(|e| PipelineError::Pdfium(format!("load: {e}")))?;
    tracing::debug!(target: "knox::render", elapsed = t1.elapsed().as_millis(), "render_pdfium_page: pdf loaded");
    let pages = document.pages();
    let page_count = pages.len() as u32;
    if page_index >= page_count {
        return Err(PipelineError::Pdfium(format!(
            "page index {page_index} out of range (0..{page_count})"
        )));
    }
    let idx: u16 = page_index.try_into().unwrap_or(u16::MAX);
    let page = pages
        .get(idx)
        .map_err(|e| PipelineError::Pdfium(format!("get page: {e}")))?;
    let rect = page.page_size();
    let pw = (rect.right() - rect.left()).value;
    let ph = (rect.top() - rect.bottom()).value;
    let dpi = dpi.clamp(72, 1200) as f32;
    let px_w = (pw * dpi / 72.0).round() as u32;
    let px_h = (ph * dpi / 72.0).round() as u32;
    if px_w == 0 || px_h == 0 {
        return Ok(None);
    }
    let target_w: i32 = px_w.try_into().unwrap_or(i32::MAX);
    let target_h: i32 = px_h.try_into().unwrap_or(i32::MAX);
    let config = PdfRenderConfig::new()
        .render_form_data(true)
        .render_annotations(true)
        .set_target_width(target_w)
        .set_target_height(target_h);
    tracing::debug!(target: "knox::render", page = page_index, "render_pdfium_page: render_with_config");
    let t2 = Instant::now();
    let bitmap = page
        .render_with_config(&config)
        .map_err(|e| PipelineError::Pdfium(format!("render: {e}")))?;
    tracing::debug!(target: "knox::render", elapsed = t2.elapsed().as_millis(), "render_pdfium_page: render done, convert to gray");
    let img = bitmap.as_image();
    let gray = img.into_luma8();
    let raw = gray.into_raw();
    let result = GrayImage::from_raw(px_w, px_h, raw)
        .ok_or_else(|| PipelineError::Pdfium("GrayImage::from_raw failed".to_string()))
        .map(Some);
    tracing::debug!(target: "knox::render", total_elapsed = t0.elapsed().as_millis(), "render_pdfium_page: complete");
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pdfium_engine_new_empty_path_returns_none() {
        let engine = PdfiumEngine::new("");
        let result = engine.render_page(Path::new("/nonexistent/test.pdf"), 0, 300);
        assert!(matches!(result, Ok(None)));
    }

    #[test]
    fn pdfium_engine_render_with_invalid_lib_returns_error() {
        let engine = PdfiumEngine::new("/nonexistent/pdfium/dylib");
        let result = engine.render_page(Path::new("/nonexistent/test.pdf"), 0, 300);
        assert!(
            result.is_err(),
            "expected error for invalid lib, got {result:?}"
        );
    }

    #[test]
    fn pdfium_engine_is_send_sync() {
        fn assert_send<T: Send + Sync>() {}
        assert_send::<PdfiumEngine>();
    }

    #[test]
    fn pdfium_engine_loads_from_env_var() {
        let var = std::env::var("PDFIUM_LIB_PATH").unwrap_or_default();
        if var.is_empty() {
            eprintln!("Skipping: set PDFIUM_LIB_PATH to test real dylib loading");
            return;
        }
        let engine = PdfiumEngine::new(&var);
        let tiny_pdf = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("tests")
            .join("fixtures")
            .join("blank.pdf");
        if tiny_pdf.exists() {
            let result = engine.render_page(&tiny_pdf, 0, 72);
            assert!(
                matches!(result, Ok(Some(_))),
                "render_page failed: {result:?}"
            );
        }
    }
}

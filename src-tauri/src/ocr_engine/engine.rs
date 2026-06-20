use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use image::GrayImage;
use rayon::prelude::*;
use tokio::sync::mpsc;

use crate::ocr_engine::error::PipelineError;
use crate::ocr_engine::image::preprocess;
use crate::ocr_engine::ingest::{IngestItem, enqueue_files};
#[cfg(feature = "ocr")]
use crate::ocr_engine::pdf::add_text_layers;
use crate::ocr_engine::pdf::{
    encode_ccitt_g4, encode_flate, extract_lopdf_page, finalize, get_page_media_box, load_document,
    replace_page_images,
};
use crate::ocr_engine::progress::ProgressTracker;
use crate::ocr_engine::render::PdfiumEngine;
use crate::ocr_engine::runtime::RuntimeResources;
use crate::ocr_engine::types::{
    CompressionMode, ExistingTextMode, OcrSettings, PipelineStatus, ProcessingConfig,
};

#[cfg(feature = "ocr")]
use crate::ocr_engine::ocr::TessApi;

#[cfg(feature = "ocr")]
pub type SharedTessPool = std::sync::Arc<std::sync::Mutex<Option<TessApi>>>;
#[cfg(not(feature = "ocr"))]
pub type SharedTessPool = std::sync::Arc<std::sync::Mutex<()>>;

/// Maximum pixel dimension on any side. Pages exceeding this are rendered at
/// a lower adaptive DPI so the resulting image fits within this cap, avoiding
/// expensive render-then-downscale waste and keeping memory bounded.
const MAX_IMAGE_DIM: u32 = 6000;

/// Orchestrates the OCR pipeline: ingests files, acquires semaphore permits,
/// and delegates per-file processing to `process_single_file`.
#[derive(Clone)]
pub struct Engine {
    runtime: std::sync::Arc<RuntimeResources>,
    #[cfg(feature = "ocr")]
    tess_pool: SharedTessPool,
    pdfium: std::sync::Arc<PdfiumEngine>,
}

impl Engine {
    /// Creates a new engine wrapping a pre-built global runtime, tess pool,
    /// and pdfium engine.
    /// The runtime (Rayon pool + semaphore) is created once in setup and
    /// shared across all jobs, avoiding repeated thread pool construction.
    pub fn new(
        runtime: std::sync::Arc<RuntimeResources>,
        #[cfg(feature = "ocr")] tess_pool: SharedTessPool,
        pdfium: std::sync::Arc<PdfiumEngine>,
    ) -> Self {
        Self {
            runtime,
            #[cfg(feature = "ocr")]
            tess_pool,
            pdfium,
        }
    }

    /// Processes all files in `items` sequentially, respecting the concurrency
    /// semaphore. Emits progress events and returns on first error or cancellation.
    pub async fn process_files(
        &self,
        app: tauri::AppHandle,
        config: ProcessingConfig,
        settings: OcrSettings,
        items: Vec<IngestItem>,
        channel_capacity: usize,
        cancelled: Arc<AtomicBool>,
    ) -> Result<(), PipelineError> {
        #[cfg(feature = "ocr")]
        let tess_pool = self.tess_pool.clone();
        let count = items.len();
        tracing::info!(target: "knox::engine", count, "processing batch");
        let (tx, mut rx) = mpsc::channel::<IngestItem>(channel_capacity);
        let tracker = ProgressTracker::new(count as u32);
        let ingest = enqueue_files(tx, items);

        tokio::spawn(async move {
            if let Err(e) = ingest.await {
                tracing::error!(target: "knox::engine", error = %e, "ingest channel failed");
            }
        });

        while let Some(item) = rx.recv().await {
            if cancelled.load(Ordering::SeqCst) {
                tracing::warn!(target: "knox::engine", "batch cancelled");
                return Err(PipelineError::Cancelled);
            }
            tracing::info!(target: "knox::engine", job_id = %item.job_id, path = %item.path.display(), "processing file");
            let semaphore = self.runtime.file_semaphore.clone();
            let permit = semaphore
                .acquire_owned()
                .await
                .map_err(|_| PipelineError::Channel("file semaphore closed".to_string()))?;
            let app = app.clone();
            let tracker = tracker.clone();
            let config = config.clone();
            let settings = settings.clone();
            let pool = self.runtime.pool.clone();
            let job_id = item.job_id.clone();
            let pdfium = self.pdfium.clone();
            let result = process_single_file(ProcessFileArgs {
                pool,
                app: app.clone(),
                tracker: tracker.clone(),
                config: config.clone(),
                settings,
                item,
                cancelled: cancelled.clone(),
                #[cfg(feature = "ocr")]
                tess_pool: tess_pool.clone(),
                pdfium,
            })
            .await;
            drop(permit);
            if let Err(err) = &result
                && !matches!(err, PipelineError::Cancelled)
            {
                tracing::error!(target: "knox::engine", job_id = %job_id, error = %err, "file processing failed");
                tracker.emit(
                    &app,
                    job_id.clone(),
                    PipelineStatus::Failed,
                    0,
                    0,
                    Some(err.to_string()),
                );
            }
            result?;
            tracing::info!(target: "knox::engine", job_id = %job_id, "file completed");
        }

        Ok(())
    }
}

/// Returns the effective page dimensions in points from the lopdf document.
/// Defaults to US Letter (612×792) if no MediaBox is found.
fn page_dimensions_pt(document: &lopdf::Document, page_number: u32) -> (f32, f32) {
    document
        .get_pages()
        .get(&page_number)
        .and_then(|&page_id| get_page_media_box(document, page_id))
        .unwrap_or((612.0, 792.0))
}

/// Computes the actual DPI to use for pdfium rendering.
/// Clamps the user's requested DPI so the rendered image fits within MAX_IMAGE_DIM,
/// avoiding expensive render-then-downscale cycles for large-format pages.
fn compute_render_dpi(user_dpi: u16, page_w_pt: f32, page_h_pt: f32) -> u16 {
    let max_dpi = MAX_IMAGE_DIM as f32 * 72.0 / page_w_pt.max(page_h_pt);
    (user_dpi as f32).min(max_dpi).max(72.0) as u16
}

fn extract_page_image(
    pdfium: &PdfiumEngine,
    document: &lopdf::Document,
    pdf_path: &Path,
    page_number: u32,
    existing_text: ExistingTextMode,
    dpi: u16,
    password: Option<&str>,
) -> Result<Option<GrayImage>, PipelineError> {
    if matches!(existing_text, ExistingTextMode::Skip)
        && let Some(&page_id) = document.get_pages().get(&page_number)
        && crate::ocr_engine::pdf::page_has_text(document, page_id).unwrap_or(false)
    {
        return Ok(None);
    }
    tracing::debug!(target: "knox::engine", page = page_number, dpi = dpi, "step: pdfium render_page");
    match pdfium.render_page(pdf_path, page_number - 1, dpi, password) {
        Ok(Some(img)) => {
            tracing::debug!(target: "knox::engine", page = page_number, w = img.width(), h = img.height(), "step: pdfium render ok");
            return Ok(Some(img));
        }
        Ok(None) => {
            tracing::debug!(target: "knox::engine", page = page_number, "step: pdfium returned None, falling back to lopdf");
        }
        Err(e) => {
            tracing::warn!(
                target: "knox::engine",
                error = %e,
                page = page_number,
                "pdfium render failed, falling back to lopdf"
            );
        }
    }
    tracing::debug!(target: "knox::engine", page = page_number, "step: lopdf extraction");
    let result = extract_lopdf_page(document, page_number, existing_text);
    match &result {
        Ok(v) => {
            tracing::debug!(target: "knox::engine", page = page_number, has_image = v.is_some(), "step: lopdf extraction done")
        }
        Err(e) => {
            tracing::warn!(target: "knox::engine", page = page_number, error = %e, "step: lopdf extraction failed")
        }
    }
    result
}

/// Context grouped to stay under clippy's 7-argument limit.
struct ProcessFileArgs {
    pool: std::sync::Arc<rayon::ThreadPool>,
    app: tauri::AppHandle,
    tracker: ProgressTracker,
    config: ProcessingConfig,
    settings: OcrSettings,
    item: IngestItem,
    cancelled: Arc<AtomicBool>,
    #[cfg(feature = "ocr")]
    tess_pool: SharedTessPool,
    pdfium: Arc<PdfiumEngine>,
}

/// Intermediate result from the parallel render+preprocess phase.
#[cfg_attr(not(feature = "ocr"), allow(dead_code))]
struct PagePrep {
    page_number: u32,
    base_image: GrayImage,
    processed: crate::ocr_engine::image::ProcessedImage,
    effective_dpi: u16,
}

async fn process_single_file(args: ProcessFileArgs) -> Result<(), PipelineError> {
    let ProcessFileArgs {
        pool,
        app,
        tracker,
        config: _config,
        settings,
        item,
        cancelled,
        pdfium,
        #[cfg(feature = "ocr")]
        tess_pool,
    } = args;
    #[cfg(feature = "ocr")]
    let config = _config;
    let input_path = &item.path;
    let job_id = item.job_id.clone();
    let output_path = item.output_path.clone();
    let do_replacement = matches!(settings.compression, CompressionMode::Ccitt);

    tracing::info!(target: "knox::engine", job_id, replacement = do_replacement, "starting file processing");
    tracker.emit(&app, job_id.clone(), PipelineStatus::Processing, 0, 0, None);

    let document = load_document(input_path, settings.password.as_deref())?;
    let total_pages = document.get_pages().len() as u32;
    tracing::info!(target: "knox::engine", job_id, total_pages, "document loaded");

    // --- Phase 1: Determine which pages to process ---
    let active_pages: Vec<u32> = (1..=total_pages)
        .filter(|&pn| {
            if matches!(settings.existing_text, ExistingTextMode::Skip)
                && let Some(&page_id) = document.get_pages().get(&pn)
                && crate::ocr_engine::pdf::page_has_text(&document, page_id).unwrap_or(false)
            {
                tracing::debug!(target: "knox::engine", job_id, page = pn, "skip: has existing text");
                return false;
            }
            true
        })
        .collect();

    if active_pages.is_empty() {
        tracing::info!(target: "knox::engine", job_id, "all pages have existing text, saving as-is");
        let mut document = document;
        finalize(&mut document, &output_path, settings.archive_enforcement)
            .map_err(|e| PipelineError::PdfParse(format!("save failed: {e}")))?;
        if !output_path.exists() {
            return Err(PipelineError::PdfParse(format!(
                "output file not found after save: {}",
                output_path.display()
            )));
        }
        tracker.emit(
            &app,
            job_id,
            PipelineStatus::Completed,
            total_pages,
            total_pages,
            None,
        );
        tracker.record_file_done();
        return Ok(());
    }

    // --- Phase 2: Parallel render + preprocess (rayon) ---
    let start = Instant::now();
    let errors_bucket: std::sync::Arc<std::sync::Mutex<Vec<(u32, String)>>> =
        std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let page_preps: Vec<PagePrep> = {
        let pool = &pool;
        let pdfium = &pdfium;
        let document = &document;
        let settings = &settings;
        let app = &app;
        let tracker = &tracker;
        let errors_bucket = errors_bucket.clone();
        pool.install(|| {
            active_pages
                .par_iter()
                .filter_map(|&page_number| {
                    if cancelled.load(Ordering::SeqCst) {
                        return None;
                    }
                    tracker.emit(
                        app,
                        job_id.clone(),
                        PipelineStatus::Processing,
                        page_number,
                        total_pages,
                        None,
                    );
                    let (page_w_pt, page_h_pt) = page_dimensions_pt(document, page_number);
                    let render_dpi = compute_render_dpi(settings.resolution_dpi, page_w_pt, page_h_pt);
                    match extract_page_image(
                        pdfium, document, input_path, page_number,
                        ExistingTextMode::Rasterize,
                        render_dpi,
                        settings.password.as_deref(),
                    ) {
                        Ok(Some(base_image)) => {
                            let max_dim = base_image.width().max(base_image.height());
                            let use_fast = max_dim > 2000
                                || base_image.pixels().map(|p| p.0[0] as u64).sum::<u64>()
                                    / base_image.width().max(1) as u64
                                    / base_image.height().max(1) as u64
                                    > 200;
                            match preprocess(&base_image, settings, use_fast) {
                                Ok(processed) => Some(PagePrep {
                                    page_number,
                                    base_image,
                                    processed,
                                    effective_dpi: render_dpi,
                                }),
                                Err(e) => {
                                    let msg = e.to_string();
                                    tracing::warn!(target: "knox::engine", job_id, page = page_number, error = %msg, "preprocess failed");
                                    if let Ok(mut bucket) = errors_bucket.lock() {
                                        bucket.push((page_number, msg));
                                    }
                                    None
                                }
                            }
                        }
                        Ok(None) => {
                            let msg = format!("page {page_number}: no image found");
                            tracing::warn!(target: "knox::engine", job_id, page = page_number, "no image found");
                            if let Ok(mut bucket) = errors_bucket.lock() {
                                bucket.push((page_number, msg));
                            }
                            None
                        }
                        Err(e) => {
                            let msg = format!("page {page_number}: {e}");
                            tracing::warn!(target: "knox::engine", job_id, page = page_number, error = %msg, "extract failed");
                            if let Ok(mut bucket) = errors_bucket.lock() {
                                bucket.push((page_number, msg));
                            }
                            None
                        }
                    }
                })
                .collect::<Vec<PagePrep>>()
                })
    };

    #[allow(unused_mut)]
    let mut page_errors = std::sync::Arc::into_inner(errors_bucket)
        .unwrap()
        .into_inner()
        .unwrap_or_default();

    if !page_errors.is_empty() && !settings.continue_on_error {
        let (pn, msg) = page_errors.into_iter().next().unwrap();
        return Err(PipelineError::PdfParse(format!("page {pn}: {msg}")));
    }
    if page_preps.is_empty() && !page_errors.is_empty() {
        return Err(PipelineError::PdfParse(format!(
            "all {} page(s) failed",
            page_errors.len(),
        )));
    }

    let prep_elapsed = start.elapsed().as_millis();
    tracing::info!(
        target: "knox::engine", job_id, pages = active_pages.len(), prepped = page_preps.len(), errors = page_errors.len(), prep_elapsed,
        "parallel render+preprocess done"
    );

    if cancelled.load(Ordering::SeqCst) {
        return Err(PipelineError::Cancelled);
    }

    #[cfg(feature = "ocr")]
    let mut text_layers: BTreeMap<u32, (Vec<crate::ocr_engine::ocr::WordBounds>, u32, u32)> =
        BTreeMap::new();
    let mut replacements: BTreeMap<u32, lopdf::Stream> = BTreeMap::new();

    #[cfg(feature = "ocr")]
    {
        // --- Phase 3: Sequential OCR (TessApi is shared) ---
        use crate::ocr_engine::ocr::TessApi;

        let tess = {
            let mut pool_guard = tess_pool
                .lock()
                .map_err(|e| PipelineError::Channel(format!("tess pool lock poisoned: {e}")))?;
            match pool_guard.take() {
                Some(tess) => {
                    tess.clear()?;
                    tracing::debug!(target: "knox::engine", job_id, "reusing warm TessApi from pool");
                    tess
                }
                None => {
                    tracing::debug!(target: "knox::engine", job_id, "creating fresh TessApi");
                    TessApi::new(&config.tessdata_path, &config.languages)?
                }
            }
        };
        tess.set_page_seg_mode(settings.psm.into())?;

        let ocr_start = Instant::now();
        for prep in &page_preps {
            if cancelled.load(Ordering::SeqCst) {
                return Err(PipelineError::Cancelled);
            }
            tracker.emit(
                &app,
                job_id.clone(),
                PipelineStatus::Ocr,
                prep.page_number,
                total_pages,
                None,
            );

            let page_result: Result<(), PipelineError> = (|| {
                let ocr_image = &prep.base_image;
                tess.set_image_bytes(
                    ocr_image.as_raw(),
                    ocr_image.width() as i32,
                    ocr_image.height() as i32,
                    1,
                    ocr_image.width() as i32,
                )?;
                tess.set_source_resolution(prep.effective_dpi as u32)?;
                let _ = tess.get_text()?;
                let min_confidence = 30.0;
                let words = tess
                    .get_words()?
                    .into_iter()
                    .filter(|w| w.confidence >= min_confidence)
                    .collect::<Vec<_>>();
                let (img_w, img_h) = (ocr_image.width(), ocr_image.height());
                text_layers.insert(prep.page_number, (words, img_w, img_h));

                if do_replacement {
                    let processed = &prep.processed;
                    let replace_image = &processed.ocr_image;
                    let stream = if let Some(ref bitonal) = processed.bitonal {
                        encode_ccitt_g4(bitonal.width, bitonal.height, bitonal.data.clone())?
                    } else {
                        encode_flate(
                            replace_image.width(),
                            replace_image.height(),
                            replace_image.to_vec(),
                        )?
                    };
                    replacements.insert(prep.page_number, stream);
                }
                Ok(())
            })();

            match page_result {
                Ok(()) => {
                    let page_elapsed = ocr_start.elapsed().as_millis() as u64;
                    tracker.record_page_time(page_elapsed);
                }
                Err(e) if settings.continue_on_error => {
                    tracing::warn!(target: "knox::engine", job_id, page = prep.page_number, error = %e, "skipping OCR page due to continue_on_error");
                    page_errors.push((prep.page_number, e.to_string()));
                }
                Err(e) => {
                    // Return TessApi to pool before propagating
                    if let Ok(mut pool_guard) = tess_pool.lock()
                        && pool_guard.is_none()
                    {
                        *pool_guard = Some(tess);
                    }
                    return Err(e);
                }
            }
        }
        let ocr_elapsed = ocr_start.elapsed().as_millis();
        tracing::info!(target: "knox::engine", job_id, ocr_elapsed, "sequential OCR done");

        if cancelled.load(Ordering::SeqCst) {
            return Err(PipelineError::Cancelled);
        }

        // --- Phase 5: Return TessApi to pool ---
        {
            let mut pool_guard = tess_pool
                .lock()
                .map_err(|e| PipelineError::Channel(format!("tess pool lock poisoned: {e}")))?;
            if pool_guard.is_none() {
                *pool_guard = Some(tess);
            }
        }
    }

    #[cfg(not(feature = "ocr"))]
    {
        // Without OCR feature: only do image replacement, no text layers
        for prep in &page_preps {
            if do_replacement {
                let processed = &prep.processed;
                let replace_image = &processed.ocr_image;
                let stream = if let Some(ref bitonal) = processed.bitonal {
                    encode_ccitt_g4(bitonal.width, bitonal.height, bitonal.data.clone())?
                } else {
                    encode_flate(
                        replace_image.width(),
                        replace_image.height(),
                        replace_image.to_vec(),
                    )?
                };
                replacements.insert(prep.page_number, stream);
            }
        }
    }

    if cancelled.load(Ordering::SeqCst) {
        return Err(PipelineError::Cancelled);
    }

    // --- Phase 4: Document modification (sequential) ---
    let mut document = document;
    if !replacements.is_empty() {
        replace_page_images(&mut document, replacements)?;
        tracing::debug!(target: "knox::engine", job_id, "image replacement done");
    }

    #[cfg(feature = "ocr")]
    if !text_layers.is_empty() {
        tracing::debug!(target: "knox::engine", job_id, "adding text layers");
        for (page_number, (words, w, h)) in &text_layers {
            let mut per_page = BTreeMap::new();
            per_page.insert(*page_number, words.clone());
            add_text_layers(&mut document, per_page, *w, *h, settings.resolution_dpi)?;
        }
        tracing::debug!(target: "knox::engine", job_id, "text layers added");
    }

    // --- Phase 6: Save ---
    finalize(&mut document, &output_path, settings.archive_enforcement)
        .map_err(|e| PipelineError::PdfParse(format!("save failed: {e}")))?;

    if !output_path.exists() {
        return Err(PipelineError::PdfParse(format!(
            "output file not found after save: {}",
            output_path.display()
        )));
    }

    tracker.record_file_done();
    tracker.emit(
        &app,
        job_id,
        PipelineStatus::Completed,
        total_pages,
        total_pages,
        None,
    );
    Ok(())
}

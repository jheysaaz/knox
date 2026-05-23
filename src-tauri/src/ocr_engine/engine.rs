use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::mpsc;

use crate::ocr_engine::error::PipelineError;
use crate::ocr_engine::image::{downsample_to_dpi, preprocess, BitonalImage};
use crate::ocr_engine::ingest::{enqueue_files, IngestItem};
use crate::ocr_engine::ocr::TessApi;
use crate::ocr_engine::pdf::{encode_ccitt_g4, encode_flate, extract_page_images, finalize, load_document, replace_page_images};
use crate::ocr_engine::progress::ProgressTracker;
use crate::ocr_engine::runtime::RuntimeResources;
use crate::ocr_engine::types::{CompressionMode, OcrSettings, PipelineStatus, ProcessingConfig};

#[derive(Clone)]
pub struct Engine {
    runtime: std::sync::Arc<RuntimeResources>,
}

impl Engine {
    pub fn new(config: &ProcessingConfig, _settings: &OcrSettings) -> Self {
        Self {
            runtime: std::sync::Arc::new(crate::ocr_engine::runtime::build_runtime(config)),
        }
    }

pub async fn process_files(
    &self,
    app: tauri::AppHandle,
    config: ProcessingConfig,
    settings: OcrSettings,
    items: Vec<IngestItem>,
    channel_capacity: usize,
    cancelled: Arc<AtomicBool>,
) -> Result<(), PipelineError> {
    let (tx, mut rx) = mpsc::channel::<IngestItem>(channel_capacity);
    let tracker = ProgressTracker::new(items.len() as u32);
    let ingest = enqueue_files(tx, items);

    tokio::spawn(async move {
        let _ = ingest.await;
    });

    while let Some(item) = rx.recv().await {
        if cancelled.load(Ordering::SeqCst) {
            return Err(PipelineError::Cancelled);
        }
        let semaphore = self.runtime.file_semaphore.clone();
        let permit = semaphore.acquire_owned().await.map_err(|_| {
            PipelineError::Channel("file semaphore closed".to_string())
        })?;
        let app = app.clone();
        let tracker = tracker.clone();
        let config = config.clone();
        let settings = settings.clone();
        let pool = self.runtime.pool.clone();
        let job_id = item.job_id.clone();
        let result = process_single_file(pool, app.clone(), tracker.clone(), config, settings, item, cancelled.clone()).await;
        drop(permit);
        if let Err(err) = result {
            if !matches!(&err, PipelineError::Cancelled) {
                tracker.emit(
                    &app,
                    job_id,
                    PipelineStatus::Failed,
                    0,
                    0,
                    Some(err.to_string()),
                );
            }
            return Err(err);
        }
    }

    Ok(())
}

}

async fn process_single_file(
    pool: std::sync::Arc<rayon::ThreadPool>,
    app: tauri::AppHandle,
    tracker: ProgressTracker,
    config: ProcessingConfig,
    settings: OcrSettings,
    item: IngestItem,
    cancelled: Arc<AtomicBool>,
) -> Result<(), PipelineError> {
    let input_path = &item.path;
    let job_id = item.job_id.clone();
    let output_path = item.output_path.clone();

    tracker.emit(&app, job_id.clone(), PipelineStatus::Processing, 0, 0, None);

    let mut document = load_document(input_path)?;
    let page_images = extract_page_images(&document, settings.existing_text.clone())?;
    let total_pages = page_images.len() as u32;

    let mut replacements: BTreeMap<u32, lopdf::Stream> = BTreeMap::new();
        let tess = TessApi::new(&config.tessdata_path, &config.languages)?;
        tess.set_page_seg_mode(settings.psm.clone().into())?;

    for page in page_images {
        if cancelled.load(Ordering::SeqCst) {
            return Err(PipelineError::Cancelled);
        }
        tracker.emit(
            &app,
            job_id.clone(),
            PipelineStatus::Ocr,
            page.page_number,
            total_pages,
            None,
        );

        let start = Instant::now();
        let base_image = downsample_to_dpi(&page.image, settings.resolution_dpi);
        let processed = pool.install(|| preprocess(&base_image, &settings))?;
        let ocr_image = processed.ocr_image;

        tess.set_image_bytes(
            ocr_image.as_raw(),
            ocr_image.width() as i32,
            ocr_image.height() as i32,
            1,
            ocr_image.width() as i32,
        )?;
        let _ = tess.get_text()?;
        tracker.record_page_time(start.elapsed().as_millis() as u64);

        tracker.emit(
            &app,
            job_id.clone(),
            PipelineStatus::Compressing,
            page.page_number,
            total_pages,
            None,
        );

        let stream = match (settings.compression.clone(), processed.bitonal) {
            (CompressionMode::Ccitt, Some(BitonalImage { data, width, height })) => {
                encode_ccitt_g4(width, height, data)
            }
            _ => encode_flate(ocr_image.width(), ocr_image.height(), ocr_image.into_raw())?,
        };
        replacements.insert(page.page_number, stream);
    }

    if cancelled.load(Ordering::SeqCst) {
        return Err(PipelineError::Cancelled);
    }
    replace_page_images(&mut document, replacements)?;
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

use serde::{Deserialize, Serialize};

/// Input from the frontend for engine-level configuration overrides.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessingConfigInput {
    /// Max number of files to process concurrently (semaphore permits).
    /// This is NOT a memory cap — it only gates file concurrency.
    /// Actual page memory is bounded by processing pages one at a time
    /// via `for_each_page_image` (streaming iterator).
    pub max_concurrent_files: Option<usize>,
    /// Path to the tessdata directory (auto-resolved if None).
    pub tessdata_path: Option<String>,
    /// Tesseract language string (e.g. "eng+spa").
    pub languages: Option<String>,
    /// Size of the Rayon thread pool for CPU-bound work.
    pub thread_pool_size: Option<usize>,
}

/// Resolved processing configuration used throughout the pipeline.
/// All optional fields from `ProcessingConfigInput` are resolved at this point.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessingConfig {
    /// Max concurrent files (semaphore permits). Engine computes default if None.
    pub max_concurrent_files: Option<usize>,
    /// Resolved absolute path to the tessdata directory.
    pub tessdata_path: String,
    /// Resolved Tesseract language string (defaults to "eng").
    pub languages: String,
    /// Rayon pool thread count (auto-computed if None).
    pub thread_pool_size: Option<usize>,
}

/// Per-job progress event sent to the frontend via `pipeline-progress`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineProgress {
    /// The job this progress event belongs to.
    pub job_id: String,
    /// Current pipeline stage.
    pub status: PipelineStatus,
    /// Current page being processed (1-indexed).
    pub current_page: u32,
    /// Total pages in the current file.
    pub total_pages: u32,
    /// Files fully processed so far across the entire queue.
    pub total_files_processed: u32,
    /// Total files in the queue when processing began.
    pub total_files_in_queue: u32,
    /// Rolling average milliseconds per page.
    pub average_ms_per_page: u64,
    /// Error message if status is Failed.
    pub error_message: Option<String>,
}

/// Stage of the OCR pipeline for progress reporting.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PipelineStatus {
    /// Loading and preprocessing the page image.
    Processing,
    /// Running Tesseract OCR recognition.
    Ocr,
    /// File completed successfully.
    Completed,
    /// File processing failed.
    Failed,
}

/// Binarization algorithm for converting grayscale to black-and-white.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BinarizationMode {
    /// Otsu's method — computes optimal threshold per page.
    Otsu,
    /// Local adaptive threshold using Bradley-Roth.
    BradleyRoth,
    /// Fixed global grayscale threshold.
    Fixed,
}

/// Deskew (rotation correction) algorithm.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeskewMode {
    /// Radon transform — best for noisy/degraded pages.
    Radon,
    /// Hough line transform — faster, works on clean text.
    Hough,
    /// Skip deskew entirely.
    Disabled,
}

/// Strategy for handling PDF pages that already contain text.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExistingTextMode {
    /// Skip OCR on pages with an existing text layer.
    Skip,
    /// Rasterize and OCR everything, overwriting existing text.
    Rasterize,
}

/// Tesseract page segmentation mode.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PageSegMode {
    /// Fully automatic page segmentation.
    Auto,
    /// Assume a single uniform block of text.
    Block,
    /// Assume a single column of text.
    Column,
    /// Treat text as sparse, unordered.
    Sparse,
}

/// Compression codec for bi-level (bitonal) page images.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CompressionMode {
    /// CCITT Group 4 fax encoding — best for binarized text.
    Ccitt,
    /// FlateDecode (Zlib) — general-purpose lossless compression.
    Flate,
}

/// Resolved OCR settings derived from frontend `OcrOptions`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrSettings {
    pub binarization: BinarizationMode,
    pub fixed_threshold: u8,
    pub deskew_mode: DeskewMode,
    pub denoise_level: u8,
    pub existing_text: ExistingTextMode,
    pub psm: PageSegMode,
    pub compression: CompressionMode,
    pub resolution_dpi: u16,
    pub archive_enforcement: bool,
}

impl From<&crate::OcrOptions> for OcrSettings {
    fn from(options: &crate::OcrOptions) -> Self {
        Self {
            binarization: options.binarization,
            fixed_threshold: options.fixed_threshold,
            deskew_mode: options.deskew_mode,
            denoise_level: options.denoise_level,
            existing_text: options.existing_text,
            psm: options.psm,
            compression: options.compression,
            resolution_dpi: options.resolution_dpi,
            archive_enforcement: options.archive_enforcement,
        }
    }
}

#[cfg(feature = "ocr")]
impl From<PageSegMode> for tesseract_sys::PageSegMode {
    fn from(mode: PageSegMode) -> Self {
        match mode {
            PageSegMode::Auto => tesseract_sys::PageSegMode::PSM_AUTO,
            PageSegMode::Block => tesseract_sys::PageSegMode::PSM_SINGLE_BLOCK,
            PageSegMode::Column => tesseract_sys::PageSegMode::PSM_SINGLE_COLUMN,
            PageSegMode::Sparse => tesseract_sys::PageSegMode::PSM_SPARSE_TEXT,
        }
    }
}

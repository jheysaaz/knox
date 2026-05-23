use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessingConfigInput {
    pub max_concurrent_files: Option<usize>,
    pub tessdata_path: Option<String>,
    pub languages: Option<String>,
    pub thread_pool_size: Option<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessingConfig {
    pub max_concurrent_files: Option<usize>,
    pub tessdata_path: String,
    pub languages: String,
    pub thread_pool_size: Option<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineProgress {
    pub job_id: String,
    pub status: PipelineStatus,
    pub current_page: u32,
    pub total_pages: u32,
    pub total_files_processed: u32,
    pub total_files_in_queue: u32,
    pub average_ms_per_page: u64,
    pub error_message: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PipelineStatus {
    Processing,
    Ocr,
    Compressing,
    Completed,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BinarizationMode {
    Otsu,
    BradleyRoth,
    Fixed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeskewMode {
    Radon,
    Hough,
    Disabled,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExistingTextMode {
    Skip,
    Rasterize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PageSegMode {
    Auto,
    Block,
    Column,
    Sparse,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CompressionMode {
    Ccitt,
    Flate,
}

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
            binarization: options.binarization.clone(),
            fixed_threshold: options.fixed_threshold,
            deskew_mode: options.deskew_mode.clone(),
            denoise_level: options.denoise_level,
            existing_text: options.existing_text.clone(),
            psm: options.psm.clone(),
            compression: options.compression.clone(),
            resolution_dpi: options.resolution_dpi,
            archive_enforcement: options.archive_enforcement,
        }
    }
}

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

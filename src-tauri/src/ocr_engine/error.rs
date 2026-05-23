use thiserror::Error;

#[derive(Debug, Error)]
pub enum PipelineError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("PDF parse error: {0}")]
    PdfParse(String),
    #[error("OCR FFI error: {0}")]
    FfiOcr(String),
    #[error("Channel error: {0}")]
    Channel(String),
    #[error("Recovered panic: {0}")]
    PanicRecovered(String),
    #[error("Cancelled")]
    Cancelled,
}

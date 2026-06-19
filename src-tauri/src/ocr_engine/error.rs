use thiserror::Error;

/// Errors that can occur during the OCR pipeline.
#[derive(Debug, Error)]
pub enum PipelineError {
    /// Wrapped I/O error from filesystem operations.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// Error parsing or writing a PDF document.
    #[error("PDF parse error: {0}")]
    PdfParse(String),
    /// Error from the Tesseract FFI layer.
    #[cfg(feature = "ocr")]
    #[error("OCR FFI error: {0}")]
    FfiOcr(String),
    /// Channel communication error in the pipeline.
    #[error("Channel error: {0}")]
    Channel(String),
    /// A panic was caught and recovered from inside an FFI call.
    #[cfg(feature = "ocr")]
    #[error("Recovered panic: {0}")]
    PanicRecovered(String),
    /// Pipeline processing was cancelled by the user.
    #[error("Cancelled")]
    Cancelled,
    /// Error from the Pdfium rendering layer. The engine falls back to lopdf
    /// extraction when this occurs, so this error is non-fatal at the page level.
    #[error("PDFium error: {0}")]
    Pdfium(String),
    /// The PDF is password-protected and no password was provided, or the
    /// provided password was incorrect.
    #[error("Encrypted PDF: {0}")]
    Encrypted(String),
}

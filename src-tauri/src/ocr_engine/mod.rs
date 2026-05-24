//! Rust-native OCR pipeline for batch PDF processing.
//!
//! The pipeline processes PDF files through stages:
//! 1. **Ingest** — Bounded channel file ingestion with backpressure
//! 2. **Load & Extract** — PDF parsing and page image extraction (lopdf)
//! 3. **Preprocess** — Denoising, binarization, deskew (image/imageproc)
//! 4. **OCR** — Tesseract FFI recognition (tesseract-sys, panic-isolated)
//! 5. **Encode** — CCITT G4 or FlateDecode compression
//! 6. **Save** — Stream replacement and PDF output
//!
//! Resource usage is throttled via a Rayon thread pool and a tokio Semaphore
//! that limits concurrent in-memory page bitmaps.

pub mod config;
pub mod engine;
pub mod error;
pub mod image;
pub mod ingest;
pub mod ocr;
pub mod pdf;
pub mod progress;
pub mod runtime;
pub mod types;
#[cfg(test)]
mod tests;

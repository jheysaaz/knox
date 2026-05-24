# Knox - Product Spec

## Overview

Offline desktop application that batch-processes PDFs using a Rust-native OCR pipeline (Tesseract FFI) to clean, OCR, and compress documents with high throughput and consistent output quality.

## Goals

- Offline, cross-platform OCR for PDF files.
- Batch processing optimized for throughput.
- PDF/A output by default (togglable in advanced panel).
- Always re-OCR (do not preserve existing text layer by default).
- Simple batch UI with a hidden advanced panel.

## Non-goals (v1)

- Cloud processing or remote jobs.
- Auto-update mechanism.
- Linux support.
- Mobile platforms.

## Supported Platforms

- macOS 12+
- Windows 10+

## Core Functional Requirements

- Multi-file batch processing with a queue and per-file progress.
- Output folder required; default output naming suffix `_cleaned`.
- OCR languages: English + Spanish.
- Compression options: CCITT Group 4 (lossless for bitonal) or FlateDecode.
- PDF/A output by default (user can toggle in advanced panel).
- Local job history (no cloud), with retention limit.
- Safe mode option in advanced panel.

## OCR Pipeline Defaults

- Languages: `eng` (single language by default, configurable)
- Binarization: Otsu adaptive thresholding
- Deskew: Radon transform
- Denoise: median filter (level 2)
- Page segmentation: fully automatic (PSM auto)
- Compression: CCITT Group 4 (for bitonal images; FlateDecode fallback)
- Resolution: 300 DPI
- PDF/A enforcement: off (togglable)

## Advanced Options

- Thread pool capacity slider.
- In-memory page cap slider.
- Binarization mode (Otsu / Bradley-Roth / Fixed).
- Fixed threshold slider.
- Deskew algorithm (Radon / Hough / Disabled).
- Despeckle intensity slider.
- Existing text handling (Skip / Rasterize).
- Tesseract page segmentation mode.
- Language string input.
- Compression codec (CCITT G4 / FlateDecode).
- Output resolution (150 / 300 / 600 DPI).
- PDF/A compliance toggle.

## Safe Mode (Advanced)

When enabled:

- Disable aggressive denoising.
- Reduce concurrency to single-file.
- Force FlateDecode compression.

## History (Offline Only)

- Local history of recent jobs.
- Retention default: 100 entries.
- No telemetry or external sync.

## Security Requirements

- No network calls.
- Strict path validation and no unintended overwrites.
- Use OS temp directories and always clean up temp artifacts.
- Run sidecar (if any) with minimal environment.
- Sanitize logs and limit export locations to user-selected output folder.

## Performance Requirements

- Throughput-optimized batching by default.
- Adaptive concurrency based on CPU and RAM.
- Stream logs to avoid large in-memory buffers.

## UX Requirements

- Simple batch screen with drag-drop and output folder selection.
- Per-file progress and status.
- Hidden advanced panel for power options.

## Bundled Dependencies

- Tesseract (via tesseract-sys FFI)
- Leptonica (via leptonica-sys)
- tessdata: eng (+ user-configurable)

## Licensing

- GPL for Tesseract/Leptonica linkage.
- Include AGPL notices for any AGPL dependencies.

# Knox - Product Spec

## Overview

Offline desktop application that batch-processes PDFs using OCRmyPDF to clean, OCR, and compress documents with high throughput and consistent output quality.

## Goals

- Offline, cross-platform OCR for PDF files.
- Batch processing optimized for throughput.
- PDF/A output by default.
- Always re-OCR (do not preserve existing text layer).
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
- Lossy compression allowed (default on).
- PDF/A output by default (user can toggle in advanced panel).
- Local job history (no cloud), with retention limit.
- Safe mode option in advanced panel.

## OCR Pipeline Defaults

- `--language eng+spa`
- `--output-type pdfa`
- `--force-ocr`
- `--optimize 3`
- Lossy compression enabled (JPEG quality ~60 by default)

## Advanced Options

- Lossy quality slider.
- Deskew.
- Clean.
- Remove background.
- Preserve metadata toggle.
- Log export toggle.
- Threading overrides (advanced).

## Safe Mode (Advanced)

When enabled:

- Disable lossy compression.
- Reduce optimization to `--optimize 1`.
- Disable aggressive cleanup (e.g., remove background).
- Force single-job concurrency.

## History (Offline Only)

- Local history of recent jobs.
- Retention default: 100 entries.
- No telemetry or external sync.

## Security Requirements

- No network calls.
- Strict path validation and no unintended overwrites.
- Use OS temp directories and always clean up temp artifacts.
- Run sidecar with minimal environment.
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

- Python runtime
- OCRmyPDF
- Tesseract
- Ghostscript
- QPDF
- Leptonica
- tessdata: eng, spa

## Licensing

- Include AGPL notices for Ghostscript and bundled dependencies.

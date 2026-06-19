# Knox

Offline desktop app for batch OCR, cleaning, and compression of PDFs using a Rust-native pipeline (Tesseract FFI).

## Features

- **Batch OCR** — Process multiple PDFs with per-file progress tracking
- **Rust-native pipeline** — Tesseract FFI via tesseract-sys, no Python sidecar
- **Image preprocessing** — Denoising, binarization (Otsu/Bradley-Roth/Fixed), deskew (Radon/Hough)
- **Compression** — CCITT Group 4 (bitonal) or FlateDecode (grayscale)
- **PDF/A support** — Optional archival compliance metadata
- **Profiles** — Balanced, Max Compression, High Fidelity, or fully Custom
- **Session logs** — View and save activity logs

## Requirements

- Node 22+
- pnpm 10+
- Rust toolchain (edition 2024)
- Tesseract + Leptonica system libs (or use bundled)

## Architecture

```
UI (React 19 + Tailwind + shadcn/ui)
  → Tauri invoke() → Rust Commands (lib.rs)
    → Queue → OCR Engine (ocr_engine/)
      → tesseract-sys (FFI) + lopdf + image/imageproc + rayon
```

## Commands

```bash
pnpm install           # Install JS deps
pnpm tauri dev         # Dev server + Tauri window
pnpm build             # TypeScript check + Vite build
pnpm test              # Vitest frontend tests (50+ tests)
cargo test             # Rust tests in src-tauri/ (75+ tests)
pnpm tauri build       # Production build
```

## Documentation

- `docs/spec.md` — Product spec
- `docs/architecture.md` — Architecture, commands, events, data flow
- `docs/BUILDING_STATIC_LIBS.md` — Building static Tesseract/Leptonica
- `docs/state-of-knox.md` — Project audit
- `docs/specs/` — Granular component/module specs
- `docs/CHANGELOG.md` — Release history
- `CONTRIBUTING.md` — Setup guide and contribution process
- `AGENTS.md` — AI agent context

## Supported Platforms

- macOS 12+ (Apple Silicon)
- Windows 10+

Offline-only; no cloud dependencies.

## Credits
- This project has some inspiration on [OCRmyPDF](https://github.com/ocrmypdf/OCRmyPDF) and also use some samples of them to test OCR. The OCRmyPDF project is lincensed under [MPL-2.0 license](https://github.com/ocrmypdf/OCRmyPDF/blob/main/LICENSE)

## License

MIT — see [LICENSE](LICENSE).

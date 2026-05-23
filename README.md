# Knox

Offline desktop app for batch OCR, cleaning, and compression of PDFs using OCRmyPDF.

## Requirements

- Node 18+
- pnpm 10+
- Rust toolchain

## Develop

```bash
pnpm install
pnpm tauri dev
```

## Notes

- macOS 12+ and Windows 10+ supported.
- Offline-only; no cloud dependencies.
- OCR binaries are bundled per platform (in progress).

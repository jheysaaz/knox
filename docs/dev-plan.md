# Development Plan (Spec-Driven)

## Phase 0 - Specs
1) Product spec: `docs/spec.md`
2) Architecture spec: `docs/architecture.md`

## Phase 1 - Scaffold
1) Tauri + React + Tailwind + shadcn/ui
2) Base layout for batch UI
3) Advanced panel + history view (local only)

## Phase 2 - Core API
1) Tauri commands: enqueue, start_queue, pause_queue, clear_queue
2) Queue state events and progress streaming
3) History storage: `history.json` under app data

## Phase 3 - OCR Sidecar Integration
1) Sidecar path resolution
2) Process spawn with secure environment
3) Progress parsing and cancellation

## Phase 4 - Packaging
1) Bundle Python + OCRmyPDF + dependencies
2) macOS 12 DMG + notarization
3) Windows 10 MSI

## Phase 5 - QA
1) Batch stress test
2) Safe mode validation
3) Offline validation

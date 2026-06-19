# Contributing to Knox

## Setup

### Prerequisites
- Node 22+, pnpm 10+
- Rust toolchain (edition 2024)
- macOS or Windows

### Install system OCR libraries

macOS:
```bash
brew install tesseract leptonica pkg-config
```

Windows (via vcpkg):
```powershell
vcpkg install tesseract leptonica
```

### Install JS dependencies
```bash
pnpm install
```

### Run development server
```bash
pnpm tauri dev
```

## Development Workflow

For every change, follow this order:

1. **Spec** — Define what needs to exist. Write/update a granular spec in `docs/specs/` with user journeys and acceptance criteria.
2. **Tests** — Define correctness before implementation. Write failing tests (Vitest for frontend, `#[cfg(test)]` for Rust).
3. **Code** — Implement just enough to make tests pass. Stick to existing patterns — don't add new frameworks or dependencies.
4. **Docs** — Document after the implementation settles. Add `///` / `/** */` doc comments on public items. Update `AGENTS.md` if commands, events, or architecture changed.
5. **Verify** — All three gates must pass before committing.

## Running Tests

### Frontend (Vitest)
```bash
pnpm test
```
- Environment: jsdom
- Library: @testing-library/react + @testing-library/user-event
- All Tauri APIs are mocked at the module level in `src/__tests__/setup.ts`

**Coverage targets per component:** all states (empty, populated, error, loading), interactions (click, type, drag), correct event handlers, type validation.

### Backend (cargo test)
```bash
cargo test --no-default-features    # Unit tests (no system OCR libs needed)
cargo test --features ocr           # With OCR feature (requires system libs)
```

Tests are placed in two locations:
- **Inline `#[cfg(test)]` modules** in each source file for unit tests
- **`tests/e2e.rs`** for integration tests (gated behind `integration` + `ocr` features)

**Key test modules:**
| Module | Tests | What it tests |
|---|---|---|
| `image.rs` | 16 | Denoise, binarize, Otsu, deskew, morphology, bitonal, fast-path |
| `pdf.rs` | 14 | Encode functions, stream dicts, finalize, decode CCITT, expand 1bpp |
| `security.rs` | 8 | Path validation, output dedup, traversal safety |
| `render.rs` | 5 | PdfiumEngine construction, empty path, invalid lib |
| `runtime.rs` | 4 | Pool + semaphore construction, Send+Sync |
| `progress.rs` | 4 | Counter increments, avg computation |
| `config.rs` | 3 | Effective concurrency calculation |

### TypeScript bindings regeneration
```bash
cargo test --features typescript --no-default-features -- export_bindings
```
Output in `src/types-gen/` (gitignored, excluded from biome).

### Verification gates
```bash
pnpm build    # biome lint + tsc + vite build
pnpm test     # frontend tests
cargo test    # Rust tests
```

All three must pass before committing.

## Code Conventions

### TypeScript/React
- Import alias `@/*` maps to `./src/*`
- Components in `src/components/`, shadcn primitives in `src/components/ui/`
- Types in `src/types.ts`
- Event listeners use `listen<T>()` from `@tauri-apps/api/event`
- Tauri commands called via `invoke()` from `@tauri-apps/api/core`

### Rust
- Snake_case for functions/variables, PascalCase for types
- `#[serde(rename_all = "camelCase")]` on all frontend-facing structs
- `#[serde(rename_all = "lowercase")]` on enums (JobStatus, OutputType)
- `PipelineError` enum with thiserror for all OCR pipeline errors
- FFI calls wrapped in `catch_unwind` for panic isolation
- Integration tests access `ocr_engine` via `knox_lib::ocr_engine::*` (module is `pub`)

### Static linking (Tesseract/Leptonica)
Prebuilt static archives go in `src-tauri/third_party/native/<target-triple>/`. See `docs/BUILDING_STATIC_LIBS.md` for instructions.

## PR Checklist
- [ ] Tests pass (`pnpm test && cargo test --no-default-features`)
- [ ] Build passes (`pnpm build`)
- [ ] No new warnings from biome or cargo
- [ ] Specs updated if behavior changed
- [ ] `AGENTS.md` updated if commands, events, or architecture changed

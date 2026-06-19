# Knox Development Roadmap

**Last updated:** June 18, 2026
**Status:** Active development

---

## Completed (v0.1.0-alpha foundation)

| Item | Description | Commit |
|---|---|---|
| Feature flag | `default = ["ocr"]`, `cargo check --no-default-features` compiles without Tesseract | — |
| History UI | Frontend `history-view.tsx` with `historyUpdated` listener, header toggle | — |
| Real CCITT G4 | `encode_ccitt_g4` rewritten with `fax` crate (real T.6 encoding) | — |
| safe_mode UI | Wired as toggle in AdvancedOptions under Hardware Allocation | — |
| Loading state | `starting` state in `useQueue`, spinner on LeftPanel button | — |

---

## Short-term

### ST-1: Refactor `commands.rs`
**Goal:** Break apart the 698-line commands file into manageable units.

**Acceptance criteria:**
- `start_queue` async loop (270 lines) extracted into at least 3 named methods on a helper struct or free functions
- Lock-poisoned error pattern (`state.lock().map_err(...)`) reduced via a helper macro or function
- `sanitize_processing_config` called once in `setup()` instead of per-job `spawn`
- All existing tests pass

**Effort:** Medium (2-4 hours)

---

### ST-2: Fix `default_concurrency()`
**Goal:** Correct the concurrency formula and document the intent.

**Acceptance criteria:**
- Formula changed from `min(2, physical_cores / 2)` to `(physical_cores / 2).clamp(1, 4)` or equivalent
- Inline comment explains the rationale
- Backend tests pass

**Effort:** Trivial (10 minutes)

---

### ST-3: Add log limit to `useLogger`
**Goal:** Prevent unbounded memory growth from long sessions.

**Acceptance criteria:**
- `useLogger` caps entries at 500
- Oldest entries are dropped when limit exceeded
- Existing `LogPanel` tests pass
- No change to `LogPanel` component API

**Effort:** Trivial (10 minutes)

---

### ST-4: Wire `fast_path=true` trigger
**Goal:** Enable the fast preprocessing path that currently has no effect.

**Acceptance criteria:**
- `fast_path` parameter (currently hardcoded `false`) is wired based on image properties:
  - `true` when image mean > 200 (already clean, skip denoise)
  - `true` when image dimensions > 2000px (skip expensive morphology at full res)
- Preprocessing tests exercise both `true` and `false` paths

**Effort:** Medium (1-2 hours)

---

### ST-5: Add TypeScript lint
**Goal:** Catch type errors and style issues before CI.

**Acceptance criteria:**
- `pnpm lint` runs a linter (Biome or ESLint) on `src/`
- `pnpm build` includes lint step (or `pnpm lint` runs separately in CI)
- CI pipeline (`ci.yml`) includes lint step

**Effort:** Medium (2-3 hours)

---

## Medium-term

### MT-6: End-to-end OCR integration test
**Goal:** Validate the full pipeline with real Tesseract.

**Acceptance criteria:**
- New test(s) in `tests/e2e.rs` gated behind `#[cfg(feature = "integration")]`
- Creates a synthetic multi-page PDF with known text
- Runs full pipeline (load → render → preprocess → OCR → encode → text layer → save)
- Asserts output PDF has searchable text layer matching input
- Requires `TESSDATA_PREFIX` env var; skips when not set
- Not run in normal `cargo test` (requires feature flag)

**Effort:** Large (4-6 hours)

---

### MT-7: Split `useQueue` hook
**Goal:** Reduce complexity of the 360-line hook.

**Acceptance criteria:**
- `useFileManager` handles: file list state, `handleFilesAdded`, `handleFileRemove`, `handleFileReprocess`, `handleClearFiles`
- `useEventListener` handles: lazy listener registration, cleanup on unmount, all Tauri event callbacks
- `useQueue` is a thin orchestrator composing the two
- All 52 frontend tests pass with minimal test changes

**Effort:** Large (3-5 hours)

---

### MT-8: Consume `queueState` event for `isRunning`
**Goal:** Use backend's authoritative state instead of derived frontend computation.

**Acceptance criteria:**
- `isRunning` derived from `queueState` event payload's `isRunning` field
- No longer computed from `files.some(f => f.status === "processing")`
- Eliminates desync between frontend/backend during pause/resume

**Effort:** Medium (1-2 hours)

---

### MT-9: History I/O on background thread
**Goal:** Prevent disk I/O from blocking queue operations.

**Acceptance criteria:**
- `save_history()` moved from inside `Mutex` lock to `tokio::task::spawn_blocking`
- Atomic write pattern preserved (write to temp, rename)
- `historyUpdated` event emitted after disk write completes

**Effort:** Medium (2-3 hours)

---

### MT-10: Proper `queueState` event handling
**Goal:** Ensure frontend fully reacts to all backend queue state changes.

**Acceptance criteria:**
- `queueState` event handler updates all relevant state (files, isRunning, queue depth)
- Tests exercise the handler with various payload shapes
- No duplicate or stale state after pause/resume/clear

**Effort:** Medium (1-2 hours)

---

## Appendix: Effort Estimates

| Item | Effort | Risk | Dependencies |
|---|---|---|---|
| ST-1: Refactor commands.rs | 2-4h | Medium | None |
| ST-2: Fix default_concurrency() | 10m | Low | None |
| ST-3: Log limit | 10m | Low | None |
| ST-4: fast_path trigger | 1-2h | Low | None |
| ST-5: TypeScript lint | 2-3h | Low | None |
| MT-6: E2E OCR test | 4-6h | High | Requires Tesseract installed |
| MT-7: Split useQueue | 3-5h | Medium | None |
| MT-8: queueState isRunning | 1-2h | Medium | MT-7 |
| MT-9: History I/O thread | 2-3h | Medium | None |
| MT-10: queueState handling | 1-2h | Low | MT-8 |

**Total remaining:** ~16-28 hours

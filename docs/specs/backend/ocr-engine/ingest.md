# Ingest Module Spec

## Struct: `IngestItem`
```rust
IngestItem {
    job_id: String,
    path: PathBuf,
    output_path: PathBuf,
}
```

## Function: `enqueue_files(tx, items) -> Result<(), PipelineError>`
- Sends each IngestItem into the bounded mpsc channel
- Returns Channel error if receiver is dropped

## Acceptance Criteria
- All items are sent successfully when channel is open
- Returns error if channel is closed early

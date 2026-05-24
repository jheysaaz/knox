# Data Flow Spec — End-to-End

## Flow: File Selection → Process → Complete

```
User drops PDFs
  → FileDropZone filters .pdf files
  → Fetches metadata (get_file_metadata)
  → Calls onFilesAdded(files)
  → App adds to files[] state

User selects output dir
  → OutputDirectory.onChange
  → App updates outputDir state

User clicks "Start OCR Processing"
  → App validates: files.length > 0, outputDir !== ""
  → Maps settings to OcrOptions + ProcessingConfigInput
  → Invoke enqueue(payload)
  → Invoke start_queue()
  → Rust spawns async processing loop

Processing loop:
  → Pops job from queue
  → Emits jobProgress (status=Running)
  → Creates OCR Engine
  → Process each page:
      Emit pipeline-progress (status=Processing → Ocr → Compressing)
  → On success: emit jobFinished (status=Completed)
     Push to history, emit historyUpdated
  → On failure: emit jobFinished (status=Failed)
     Push error to history
  → On cancel: emit jobFinished (status=Cancelled)
     No history entry

UI handles events:
  → pipeline-progress: update file progress %
  → jobFinished: mark file complete/error/paused, add log
  → queueState: sync file statuses
  → historyUpdated: (listener ready for future history UI)
```

## Error Paths
1. **No files**: Toast "No files in queue" — no command called
2. **No output dir**: Toast "No output directory selected" — no command called
3. **Enqueue fails**: Toast error message, add error log
4. **Processing fails per file**: That file marked "Error", others continue
5. **Cancel during processing**: Current file stops, remaining stay queued

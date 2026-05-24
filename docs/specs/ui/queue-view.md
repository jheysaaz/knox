# QueueView — File Queue Display Component

## User Journey
1. User sees a card titled "Queue" with a list of added files
2. Each file shows: icon, name, size, status label
3. Processing files show a progress bar
4. Completed/errored/paused files show a Reprocess button
5. All files have a Remove (X) button
6. A Clear button empties the queue (when no files are processing)
7. A Pause button appears when the queue is running

## Props
- `files: FileItem[]` — list of files
- `onFileRemove: (id: string) => void` — remove single file
- `onClear: () => void` — clear all files
- `onReprocess?: (id: string) => void` — re-queue a file
- `onStop?: () => void` — pause the queue
- `isRunning?: boolean` — whether queue is active

## Status Rendering
| Status | Icon | Label | Color |
|---|---|---|---|
| pending | FileText | "Pending" | muted |
| processing (running) | Loader2 spin | "Processing..." | blue |
| processing (paused) | PauseCircle | "Pausing..." | amber |
| complete | CheckCircle2 | "Complete" | green |
| error | AlertCircle | "Error" | destructive |
| paused | Ban | "Paused" | amber |

## Acceptance Criteria
- Empty state shows "No files added yet"
- Each file shows icon, name, size, and status
- Progress bar renders for processing files
- Remove button calls onFileRemove with file id
- Reprocess button shows for completed/error/paused files
- Clear button calls onClear (disabled when processing)
- Pause button shows when isRunning=true
- File sizes format correctly (B, KB, MB)

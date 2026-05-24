# FileDropZone — File Selection Component

## User Journey
1. User sees a dashed drop zone with upload icon
2. User can click to browse files via OS dialog
3. User can drag-and-drop files onto the zone
4. Only `.pdf` files are accepted; non-PDFs are silently filtered
5. File metadata (size) is fetched via Tauri command

## Props
- `onFilesAdded: (files: FileItem[]) => void` — called when valid files are selected

## Behavior
- Clicking zone opens `open()` dialog with PDF filter
- Drag events handled via Tauri `onDragDropEvent`
- Filters non-PDF files before calling `onFilesAdded`
- Calls `get_file_metadata` for each file to get size

## States
- **Default**: Dashed border, "Drop PDF files here" + "or click to browse"
- **Dragging**: Solid border, scaled upload icon, "Drop files here"
- **Error**: Silent failure (console.error for dialog errors)

## Acceptance Criteria
- Click triggers file dialog with PDF filter
- Only .pdf files are passed to onFilesAdded
- Dialog cancellation returns without action
- File metadata is fetched for each accepted file
- Drag state toggles visual feedback

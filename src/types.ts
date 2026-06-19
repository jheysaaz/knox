/** A single file entry displayed in the queue view. */
export interface FileItem {
  id: string;
  path: string;
  name: string;
  size: number;
  status: 'pending' | 'processing' | 'complete' | 'error' | 'paused';
  progress?: number;
  queued?: boolean;
}

/** Binarization algorithm for converting grayscale to black/white. */
export type BinarizationMode = 'otsu' | 'bradley-roth' | 'fixed';
/** Deskew correction strategy. */
export type DeskewMode = 'radon' | 'hough' | 'disabled';
/** How to handle pages that already contain text. */
export type ExistingTextMode = 'skip' | 'rasterize';
/** Tesseract page segmentation mode. */
export type PageSegMode = 'auto' | 'block' | 'column' | 'sparse';
/** Compression codec for output image streams. */
export type CompressionMode = 'ccitt' | 'flate';

/** Full set of OCR pipeline options passed from the UI to the Rust backend. */
export interface OcrSettings {
  memoryPages: number;
  binarization: BinarizationMode;
  fixedThreshold: number;
  deskew: DeskewMode;
  denoiseLevel: number;
  existingText: ExistingTextMode;
  psm: PageSegMode;
  compression: CompressionMode;
  resolution: string;
  archiveEnforcement: boolean;
  languages: string;
}

/** A single log line displayed in the activity panel. */
export interface LogEntry {
  id: string;
  timestamp: Date;
  level: 'info' | 'warn' | 'error';
  message: string;
}

/** A completed job entry in the processing history. */
export interface HistoryEntry {
  id: string;
  inputPath: string;
  outputPath: string;
  status: 'completed' | 'failed' | 'cancelled';
  startedAt: number;
  finishedAt: number;
  durationMs: number;
}

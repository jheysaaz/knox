// ── IPC types consumed by frontend code ───────────────────────────────────
// Mirrors Rust types in lib.rs and ocr_engine/types.rs (camelCase matches
// serde rename_all on the Rust side).

export type JobStatus =
  | 'queued'
  | 'running'
  | 'completed'
  | 'failed'
  | 'cancelled';

export type PipelineStatus = 'processing' | 'ocr' | 'completed' | 'failed';

export type BinarizationMode = 'otsu' | 'bradley-roth' | 'fixed';

export type CompressionMode = 'ccitt' | 'flate';

export type DeskewMode = 'radon' | 'hough' | 'disabled';

export type ExistingTextMode = 'skip' | 'rasterize';

export type OutputType = 'pdfa' | 'pdf';

export type PageSegMode = 'auto' | 'block' | 'column' | 'sparse';

export interface OcrOptions {
  outputType: OutputType;
  safeMode: boolean;
  maxConcurrency: number | null;
  binarization: BinarizationMode;
  fixedThreshold: number;
  deskewMode: DeskewMode;
  denoiseLevel: number;
  existingText: ExistingTextMode;
  psm: PageSegMode;
  compression: CompressionMode;
  resolutionDpi: number;
  archiveEnforcement: boolean;
  languages: string | null;
  memoryPages: number | null;
  continueOnError: boolean;
  password: string | null;
}

export interface ProcessingConfigInput {
  maxConcurrentFiles: number | null;
  tessdataPath: string | null;
  languages: string | null;
  threadPoolSize: number | null;
}

export interface Job {
  id: string;
  inputPath: string;
  outputPath: string;
  status: JobStatus;
  percent: number;
  startedAt: number | null;
  finishedAt: number | null;
  options: OcrOptions;
  processing: ProcessingConfigInput | null;
  errorMessage: string | null;
}

export interface QueueState {
  jobs: Array<Job>;
  isRunning: boolean;
}

export interface PipelineProgress {
  jobId: string;
  status: PipelineStatus;
  currentPage: number;
  totalPages: number;
  totalFilesProcessed: number;
  totalFilesInQueue: number;
  averageMsPerPage: number;
  errorMessage: string | null;
}

export interface HistoryEntry {
  id: string;
  inputPath: string;
  outputPath: string;
  status: JobStatus;
  startedAt: number;
  finishedAt: number;
  durationMs: number;
  options: OcrOptions;
}

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

/** A single log line displayed in the activity panel. */
export interface LogEntry {
  id: string;
  timestamp: Date;
  level: 'info' | 'warn' | 'error';
  message: string;
}

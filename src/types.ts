// Re-export all IPC types generated from Rust via ts-rs.
import type {
  BinarizationMode,
  CompressionMode,
  DeskewMode,
  ExistingTextMode,
  FileMetadata,
  HistoryEntry,
  Job,
  JobStatus,
  OcrOptions,
  OutputType,
  PageSegMode,
  PipelineProgress,
  PipelineStatus,
  ProcessingConfigInput,
  QueueState,
} from '@/types-gen/index';

export type {
  BinarizationMode,
  CompressionMode,
  DeskewMode,
  ExistingTextMode,
  FileMetadata,
  HistoryEntry,
  Job,
  JobStatus,
  OcrOptions,
  OutputType,
  PageSegMode,
  PipelineProgress,
  PipelineStatus,
  ProcessingConfigInput,
  QueueState,
};

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

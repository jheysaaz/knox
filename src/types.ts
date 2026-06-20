// IPC types consumed by frontend code. Generated from Rust via ts-rs.
import type {
  HistoryEntry,
  Job,
  PipelineProgress,
  QueueState,
} from '@/types-gen/index';

export type { HistoryEntry, Job, PipelineProgress, QueueState };

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

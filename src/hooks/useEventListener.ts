import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useCallback, useEffect, useRef, useState } from 'react';
import type { FileItem, HistoryEntry, LogEntry } from '@/types';

interface QueueState {
  jobs: Job[];
  isRunning: boolean;
}

interface Job {
  id: string;
  inputPath: string;
  outputPath: string;
  status: 'queued' | 'running' | 'completed' | 'failed' | 'cancelled';
  percent: number;
  startedAt?: number;
  finishedAt?: number;
  errorMessage?: string | null;
}

interface PipelineProgress {
  jobId: string;
  status: 'processing' | 'ocr' | 'completed' | 'failed';
  currentPage: number;
  totalPages: number;
  totalFilesProcessed: number;
  totalFilesInQueue: number;
  averageMsPerPage: number;
  errorMessage?: string | null;
}

const mapJobStatus = (status: Job['status']): FileItem['status'] => {
  switch (status) {
    case 'running':
      return 'processing';
    case 'completed':
      return 'complete';
    case 'failed':
      return 'error';
    case 'cancelled':
      return 'paused';
    default:
      return 'pending';
  }
};

export function useEventListener(
  setFiles: React.Dispatch<React.SetStateAction<FileItem[]>>,
  addLog: (level: LogEntry['level'], message: string) => void,
  onEncryptionError?: (filePath: string) => void,
) {
  const [history, setHistory] = useState<HistoryEntry[]>([]);
  const [isRunning, setIsRunning] = useState(false);

  const listenersReady = useRef(false);
  const cleanupFns = useRef<(() => void)[]>([]);
  const onEncryptionErrorRef = useRef(onEncryptionError);
  onEncryptionErrorRef.current = onEncryptionError;

  useEffect(() => {
    invoke<HistoryEntry[]>('get_history')
      .then(setHistory)
      .catch(() => {});
    return () => {
      cleanupFns.current.forEach((fn) => fn());
    };
  }, []);

  const ensureListeners = useCallback(async () => {
    if (listenersReady.current) return;
    listenersReady.current = true;
    const fns = await Promise.all([
      listen<PipelineProgress>('pipeline-progress', (event) => {
        const progress = event.payload;
        if (progress.status === 'failed' && progress.errorMessage) {
          addLog('error', progress.errorMessage);
        }
        setFiles((prev) =>
          prev.map((file) => {
            if (file.id !== progress.jobId) return file;
            const percent = progress.totalPages
              ? Math.round((progress.currentPage / progress.totalPages) * 100)
              : 0;
            return {
              ...file,
              status:
                progress.status === 'failed'
                  ? 'error'
                  : progress.status === 'completed'
                    ? 'complete'
                    : 'processing',
              progress: percent,
            };
          }),
        );
      }),
      listen<QueueState>('queueState', (event) => {
        const snapshot = event.payload;
        setIsRunning(snapshot.isRunning);
        const backendIds = new Set(snapshot.jobs.map((j) => j.id));
        setFiles((prev) => {
          const filtered = prev.filter((file) => {
            if (!file.queued) return true;
            if (backendIds.has(file.id)) return true;
            return false;
          });
          return filtered.map((file) => {
            const job = snapshot.jobs.find((j) => j.id === file.id);
            if (!job) return file;
            return {
              ...file,
              status: mapJobStatus(job.status),
              queued: true,
            };
          });
        });
      }),
      listen<Job>('jobFinished', (event) => {
        const job = event.payload;
        setFiles((prev) =>
          prev.map((file) =>
            file.id === job.id
              ? {
                  ...file,
                  status:
                    job.status === 'completed'
                      ? 'complete'
                      : job.status === 'cancelled'
                        ? 'paused'
                        : 'error',
                  progress: job.status === 'completed' ? 100 : file.progress,
                  queued: true,
                }
              : file,
          ),
        );
        addLog(
          job.status === 'completed'
            ? 'info'
            : job.status === 'cancelled'
              ? 'warn'
              : 'error',
          job.status === 'completed'
            ? `Completed: ${job.inputPath} → ${job.outputPath}`
            : job.status === 'cancelled'
              ? `Paused: ${job.inputPath}`
              : `Failed: ${job.errorMessage || job.inputPath}`,
        );
        if (
          job.status === 'failed' &&
          job.errorMessage &&
          (job.errorMessage.toLowerCase().includes('password') ||
            job.errorMessage.toLowerCase().includes('encrypt'))
        ) {
          onEncryptionErrorRef.current?.(job.inputPath);
        }
      }),
      listen<Job>('jobProgress', (event) => {
        const job = event.payload;
        setFiles((prev) =>
          prev.map((file) =>
            file.id === job.id
              ? { ...file, status: mapJobStatus(job.status) }
              : file,
          ),
        );
      }),
      listen<HistoryEntry[]>('historyUpdated', (event) => {
        setHistory(event.payload);
      }),
    ]);
    cleanupFns.current = fns;

    try {
      const initial = await invoke<HistoryEntry[]>('get_history');
      setHistory(initial);
    } catch {
      // history load is best-effort
    }
  }, [addLog, setFiles]);

  const handleClearHistory = useCallback(async () => {
    try {
      await invoke('clear_history');
      setHistory([]);
      addLog('info', 'History cleared');
    } catch {
      // best-effort
    }
  }, [addLog]);

  return {
    history,
    isRunning,
    setIsRunning,
    ensureListeners,
    handleClearHistory,
  };
}

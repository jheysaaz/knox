import { invoke } from '@tauri-apps/api/core';
import { useCallback, useState } from 'react';
import { toast } from 'sonner';
import type { FileItem, LogEntry } from '@/types';

export function useFileManager(
  addLog: (level: LogEntry['level'], message: string) => void,
) {
  const [files, setFiles] = useState<FileItem[]>([]);

  const handleFilesAdded = useCallback((newFiles: FileItem[]) => {
    setFiles((prev) =>
      [...prev, ...newFiles].map((file) => ({
        ...file,
        queued: file.queued ?? false,
      })),
    );
  }, []);

  const handleFileRemove = useCallback(
    async (id: string) => {
      const file = files.find((f) => f.id === id);
      if (!file) return;
      if (file.status === 'processing') {
        toast.error('Cannot remove a file that is currently processing');
        return;
      }
      if (!file.queued) {
        setFiles((prev) => prev.filter((f) => f.id !== id));
        addLog('info', `Removed: ${file.name}`);
        return;
      }
      try {
        await invoke('remove_job', { job_id: id });
        setFiles((prev) => prev.filter((f) => f.id !== id));
        addLog('info', `Removed: ${file.name}`);
      } catch {
        toast.error('Unable to remove file');
      }
    },
    [files, addLog],
  );

  const handleFileReprocess = useCallback(
    async (id: string) => {
      const file = files.find((f) => f.id === id);
      if (!file) return;

      if (file.queued) {
        try {
          await invoke('remove_job', { job_id: id });
        } catch {
          // job may already be gone from backend
        }
      }

      setFiles((prev) =>
        prev.map((f) =>
          f.id === id
            ? {
                ...f,
                status: 'pending' as const,
                queued: false,
                progress: undefined,
              }
            : f,
        ),
      );
      addLog('info', `Queued for reprocess: ${file.name}`);
    },
    [files, addLog],
  );

  const handleClearFiles = useCallback(async () => {
    try {
      await invoke('clear_queue');
      setFiles([]);
      addLog('info', 'Queue cleared');
    } catch {
      toast.error('Unable to clear queue');
    }
  }, [addLog]);

  return {
    files,
    setFiles,
    handleFilesAdded,
    handleFileRemove,
    handleFileReprocess,
    handleClearFiles,
  };
}

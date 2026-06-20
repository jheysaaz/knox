import { invoke } from '@tauri-apps/api/core';
import { useCallback, useRef, useState } from 'react';
import { toast } from 'sonner';
import type { ProfileValues } from '@/components/advanced-options';
import type { LogEntry, QueueState } from '@/types';
import { useEventListener } from './useEventListener';
import { useFileManager } from './useFileManager';

interface FileEncryptionInfo {
  encrypted: boolean;
  fileId: string;
}

const mapSettingsToOptions = (values: ProfileValues, password?: string) => ({
  outputType: values.archiveEnforcement ? 'pdfa' : 'pdf',
  safeMode: values.safeMode,
  binarization: values.binarization,
  fixedThreshold: values.fixedThreshold,
  deskewMode: values.deskew,
  denoiseLevel: values.denoiseLevel,
  existingText: values.existingText,
  psm: values.psm,
  compression: values.compression,
  resolutionDpi: Number(values.resolution),
  archiveEnforcement: values.archiveEnforcement,
  languages: values.languages.join('+'),
  memoryPages: values.memoryPages,
  continueOnError: values.continueOnError,
  password: password || undefined,
});

const mapSettingsToProcessing = (values: ProfileValues) => ({
  maxConcurrentFiles: values.memoryPages,
  tessdataPath: undefined,
  languages: values.languages.join('+'),
});

export function useQueue(
  addLog: (level: LogEntry['level'], message: string) => void,
  onJobError?: (filePath: string, errorMessage: string) => void,
) {
  const {
    files,
    setFiles,
    handleFilesAdded: baseHandleFilesAdded,
    handleFileRemove,
    handleFileReprocess,
    handleClearFiles,
  } = useFileManager(addLog);

  const [outputDir, setOutputDir] = useState('');
  const [starting, setStarting] = useState(false);

  const [passwordDialogOpen, setPasswordDialogOpen] = useState(false);
  const [pendingEncryptedFiles, setPendingEncryptedFiles] = useState<
    { id: string; name: string; path: string }[]
  >([]);
  const passwordCache = useRef<Map<string, string>>(new Map());
  const encryptedFiles = useRef<Set<string>>(new Set());
  const pendingStartSettings = useRef<ProfileValues | null>(null);
  const lastStartSettings = useRef<ProfileValues | null>(null);

  const handleEncryptionError = useCallback(
    (filePath: string) => {
      passwordCache.current.delete(filePath);
      const file = files.find((f) => f.path === filePath);
      if (file) {
        setFiles((prev) =>
          prev.map((f) =>
            f.path === filePath
              ? {
                  ...f,
                  status: 'pending' as const,
                  queued: false,
                  progress: undefined,
                }
              : f,
          ),
        );
        if (lastStartSettings.current) {
          pendingStartSettings.current = lastStartSettings.current;
        }
        setPendingEncryptedFiles([
          { id: file.id, name: file.name, path: file.path },
        ]);
        setPasswordDialogOpen(true);
      }
    },
    [files, setFiles],
  );

  const {
    history,
    isRunning,
    setIsRunning,
    ensureListeners,
    handleClearHistory,
  } = useEventListener(setFiles, addLog, handleEncryptionError, onJobError);

  const executeStart = useCallback(
    async (settings: ProfileValues) => {
      const pending = files.filter(
        (file) => file.status === 'pending' && !file.queued,
      );
      if (pending.length === 0) {
        toast.error('No pending files to process');
        return;
      }

      setStarting(true);
      try {
        if (settings.languages.length > 0) {
          const langResult = await invoke<{
            downloaded: string[];
            skipped: string[];
            errors: Record<string, string>;
          }>('ensure_language_packs', {
            languages: [...new Set(settings.languages)],
          });
          if (langResult.downloaded.length > 0) {
            addLog(
              'info',
              `Downloaded ${langResult.downloaded.length} language pack(s)`,
            );
          }
          if (Object.keys(langResult.errors).length > 0) {
            const failed = Object.entries(langResult.errors)
              .map(([l, e]) => `${l}: ${e}`)
              .join('; ');
            addLog('warn', `Language pack download issues: ${failed}`);
          }
        }

        const paths = pending.map((file) => file.path);
        const passwords = paths.map(
          (p) => passwordCache.current.get(p) || undefined,
        );
        const allSame = passwords.every((p, _i, a) => p === a[0]);

        if (allSame) {
          const options = mapSettingsToOptions(settings, passwords[0]);
          const processing = mapSettingsToProcessing(settings);
          const state = await invoke<QueueState>('enqueue', {
            payload: {
              files: paths,
              outputDir,
              options,
              processing,
            },
          });
          setIsRunning(state.isRunning);
          setFiles((prev) => {
            const used = new Set<string>();
            const newJobs = state.jobs.filter((j) => j.status === 'queued');
            return prev.map((file) => {
              if (file.status !== 'pending' || file.queued) return file;
              const job = newJobs.find(
                (j) => j.inputPath === file.path && !used.has(j.id),
              );
              if (!job) return file;
              used.add(job.id);
              return { ...file, id: job.id, queued: true };
            });
          });
        } else {
          for (let i = 0; i < paths.length; i++) {
            const options = mapSettingsToOptions(settings, passwords[i]);
            const processing = mapSettingsToProcessing(settings);
            await invoke<QueueState>('enqueue', {
              payload: {
                files: [paths[i]],
                outputDir,
                options,
                processing,
              },
            });
          }
          await invoke<QueueState>('get_status');
          setFiles((prev) =>
            prev.map((f) =>
              pending.some((p) => p.id === f.id) ? { ...f, queued: true } : f,
            ),
          );
        }

        await ensureListeners();
        await invoke('start_queue');
        addLog('info', `Processing ${paths.length} file(s)...`);
      } catch (err) {
        const message = (err as { message?: string })?.message || String(err);
        toast.error(`Failed to start processing: ${message}`);
        addLog('error', `Start failed: ${message}`);
      } finally {
        setStarting(false);
      }
    },
    [files, outputDir, addLog, setFiles, setIsRunning, ensureListeners],
  );

  const handleFilesAdded = useCallback(
    async (
      newFiles: {
        id: string;
        path: string;
        name: string;
        size: number;
        status: string;
        queued?: boolean;
      }[],
    ) => {
      baseHandleFilesAdded(newFiles as any);

      const checks = await Promise.allSettled(
        newFiles.map(async (file) => {
          const info = await invoke<FileEncryptionInfo>(
            'check_file_encrypted',
            {
              path: file.path,
            },
          );
          return { file, info };
        }),
      );

      const encrypted: { id: string; name: string; path: string }[] = [];
      for (const result of checks) {
        if (result.status === 'fulfilled' && result.value.info.encrypted) {
          encryptedFiles.current.add(result.value.file.path);
          const cached = passwordCache.current.get(result.value.file.path);
          if (!cached) {
            encrypted.push({
              id: result.value.file.id,
              name: result.value.file.name,
              path: result.value.file.path,
            });
          }
        }
      }

      if (encrypted.length > 0) {
        setPendingEncryptedFiles(encrypted);
        setPasswordDialogOpen(true);
      }
    },
    [baseHandleFilesAdded],
  );

  const handlePasswordConfirm = useCallback(
    (password: string) => {
      for (const file of pendingEncryptedFiles) {
        passwordCache.current.set(file.path, password);
      }
      setPendingEncryptedFiles([]);
      setPasswordDialogOpen(false);

      if (pendingStartSettings.current) {
        executeStart(pendingStartSettings.current);
      }
    },
    [pendingEncryptedFiles, executeStart],
  );

  const handlePasswordCancel = useCallback(() => {
    pendingStartSettings.current = null;
    setPendingEncryptedFiles([]);
    setPasswordDialogOpen(false);
  }, []);

  const handleStart = useCallback(
    async (settings: ProfileValues) => {
      if (files.length === 0) {
        toast.error('No files in queue');
        return;
      }
      if (!outputDir) {
        toast.error('No output directory selected');
        return;
      }

      const pending = files.filter(
        (file) => file.status === 'pending' && !file.queued,
      );
      if (pending.length === 0) {
        toast.error('No pending files to process');
        return;
      }

      const encryptedWithoutPassword = pending.filter(
        (f) =>
          encryptedFiles.current.has(f.path) &&
          !passwordCache.current.has(f.path),
      );
      lastStartSettings.current = settings;
      if (encryptedWithoutPassword.length > 0) {
        pendingStartSettings.current = settings;
        setPendingEncryptedFiles(
          encryptedWithoutPassword.map((f) => ({
            id: f.id,
            name: f.name,
            path: f.path,
          })),
        );
        setPasswordDialogOpen(true);
        return;
      }

      await executeStart(settings);
    },
    [files, outputDir, executeStart],
  );

  const handleStop = useCallback(async () => {
    try {
      await invoke('pause_queue');
      addLog('info', 'Queue paused');
    } catch {
      toast.error('Unable to cancel queue');
    }
  }, [addLog]);

  return {
    files,
    outputDir,
    setOutputDir,
    isRunning,
    starting,
    history,
    handleFilesAdded,
    handleFileRemove,
    handleFileReprocess,
    handleClearFiles,
    handleStart,
    handleStop,
    handleClearHistory,
    passwordDialogOpen,
    pendingEncryptedFiles,
    handlePasswordConfirm,
    handlePasswordCancel,
  };
}

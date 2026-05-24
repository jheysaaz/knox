import { useState, useEffect, useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { toast } from "sonner";
import type { FileItem, LogEntry } from "@/types";
import type { ProfileValues } from "@/components/advanced-options";

interface QueueState {
  jobs: Job[];
  isRunning: boolean;
}

interface Job {
  id: string;
  inputPath: string;
  outputPath: string;
  status: "queued" | "running" | "completed" | "failed" | "cancelled";
  percent: number;
  startedAt?: number;
  finishedAt?: number;
  errorMessage?: string | null;
}

interface PipelineProgress {
  jobId: string;
  status: "processing" | "ocr" | "compressing" | "completed" | "failed";
  currentPage: number;
  totalPages: number;
  totalFilesProcessed: number;
  totalFilesInQueue: number;
  averageMsPerPage: number;
  errorMessage?: string | null;
}

const mapJobStatus = (status: Job["status"]): FileItem["status"] => {
  switch (status) {
    case "running":
      return "processing";
    case "completed":
      return "complete";
    case "failed":
      return "error";
    case "cancelled":
      return "paused";
    default:
      return "pending";
  }
};

const mapSettingsToOptions = (values: ProfileValues) => ({
  outputType: values.archiveEnforcement ? "pdfa" : "pdf",
  lossyCompression: false,
  jpegQuality: 60,
  deskew: values.deskew !== "disabled",
  clean: values.denoiseLevel > 0,
  removeBackground: values.binarization !== "fixed",
  preserveMetadata: true,
  safeMode: false,
  maxConcurrency: values.cpuCores,
  perJobThreads: values.cpuCores,
  binarization: values.binarization,
  fixedThreshold: values.fixedThreshold,
  deskewMode: values.deskew,
  denoiseLevel: values.denoiseLevel,
  existingText: values.existingText,
  psm: values.psm,
  compression: values.compression,
  resolutionDpi: Number(values.resolution),
  archiveEnforcement: values.archiveEnforcement,
  languages: values.languages,
  memoryPages: values.memoryPages,
});

const mapSettingsToProcessing = (values: ProfileValues) => ({
  maxConcurrentFiles: values.memoryPages,
  tessdataPath: undefined,
  languages: values.languages,
  threadPoolSize: values.cpuCores,
});

export function useQueue(addLog: (level: LogEntry["level"], message: string) => void) {
  const [files, setFiles] = useState<FileItem[]>([]);
  const [outputDir, setOutputDir] = useState("");
  const [showActivity, setShowActivity] = useState(true);

  const isRunning = useMemo(
    () => files.some((f) => f.status === "processing"),
    [files],
  );

  useEffect(() => {
    const load = async () => {
      try {
        const state = await invoke<QueueState>("get_status");
        if (state.jobs.length > 0) {
          setFiles((prev) => {
            if (prev.length > 0) return prev;
            return state.jobs.map((job) => ({
              id: job.id,
              path: job.inputPath,
              name: job.inputPath.split("/").pop() || job.inputPath,
              size: 0,
              status: mapJobStatus(job.status),
              queued: true,
            }));
          });
        }
      } catch {
        // ignore
      }
    };
    load();
  }, []);

  useEffect(() => {
    let unlistenProgress: (() => void) | undefined;
    let unlistenQueue: (() => void) | undefined;
    let unlistenFinish: (() => void) | undefined;
    let unlistenJobProgress: (() => void) | undefined;

    const setup = async () => {
      unlistenProgress = await listen<PipelineProgress>(
        "pipeline-progress",
        (event) => {
          const progress = event.payload;
          if (progress.status === "failed" && progress.errorMessage) {
            addLog("error", progress.errorMessage);
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
                  progress.status === "failed"
                    ? "error"
                    : progress.status === "completed"
                      ? "complete"
                      : "processing",
                progress: percent,
              };
            }),
          );
        },
      );
      unlistenQueue = await listen<QueueState>("queueState", (event) => {
        const snapshot = event.payload;
        setFiles((prev) =>
          prev.map((file) => {
            const job = snapshot.jobs.find((j) => j.id === file.id);
            if (!job) return file;
            return {
              ...file,
              status: mapJobStatus(job.status),
              queued: true,
            };
          }),
        );
      });
      unlistenFinish = await listen<Job>("jobFinished", (event) => {
        const job = event.payload;
        setFiles((prev) =>
          prev.map((file) =>
            file.id === job.id
              ? {
                  ...file,
                  status:
                    job.status === "completed"
                      ? "complete"
                      : job.status === "cancelled"
                        ? "paused"
                        : "error",
                  progress: job.status === "completed" ? 100 : file.progress,
                  queued: true,
                }
              : file,
          ),
        );
        addLog(
          job.status === "completed"
            ? "info"
            : job.status === "cancelled"
              ? "warn"
              : "error",
          job.status === "completed"
            ? `Completed: ${job.inputPath} → ${job.outputPath}`
            : job.status === "cancelled"
              ? `Paused: ${job.inputPath}`
              : `Failed: ${job.errorMessage || job.inputPath}`,
        );
      });
      unlistenJobProgress = await listen<Job>("jobProgress", (event) => {
        const job = event.payload;
        setFiles((prev) =>
          prev.map((file) =>
            file.id === job.id
              ? { ...file, status: mapJobStatus(job.status) }
              : file,
          ),
        );
      });
    };

    setup();
    return () => {
      if (unlistenProgress) unlistenProgress();
      if (unlistenQueue) unlistenQueue();
      if (unlistenFinish) unlistenFinish();
      if (unlistenJobProgress) unlistenJobProgress();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handleFilesAdded = (newFiles: FileItem[]) => {
    setFiles((prev) =>
      [...prev, ...newFiles].map((file) => ({
        ...file,
        queued: file.queued ?? false,
      })),
    );
    addLog("info", `${newFiles.length} file(s) added`);
  };

  const handleFileRemove = useCallback(
    async (id: string) => {
      const file = files.find((f) => f.id === id);
      if (!file) return;
      if (file.status === "processing") {
        toast.error("Cannot remove a file that is currently processing");
        return;
      }
      if (!file.queued) {
        setFiles((prev) => prev.filter((f) => f.id !== id));
        addLog("info", `Removed: ${file.name}`);
        return;
      }
      try {
        await invoke("remove_job", { job_id: id });
        setFiles((prev) => prev.filter((f) => f.id !== id));
        addLog("info", `Removed: ${file.name}`);
      } catch {
        toast.error("Unable to remove file");
      }
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [files],
  );

  const handleFileReprocess = useCallback(
    async (id: string) => {
      const file = files.find((f) => f.id === id);
      if (!file) return;

      if (file.queued) {
        try {
          await invoke("remove_job", { job_id: id });
        } catch {
          // job may already be gone from backend
        }
      }

      setFiles((prev) =>
        prev.map((f) =>
          f.id === id
            ? { ...f, status: "pending", queued: false, progress: undefined }
            : f,
        ),
      );
      addLog("info", `Queued for reprocess: ${file.name}`);
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [files],
  );

  const handleClearFiles = useCallback(async () => {
    try {
      await invoke("clear_queue");
      setFiles([]);
      addLog("info", "Queue cleared");
    } catch {
      toast.error("Unable to clear queue");
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handleStart = async (settings: ProfileValues) => {
    if (files.length === 0) {
      toast.error("No files in queue");
      return;
    }
    if (!outputDir) {
      toast.error("No output directory selected");
      return;
    }
    try {
      const options = mapSettingsToOptions(settings);
      const processing = mapSettingsToProcessing(settings);
      const pending = files.filter(
        (file) => file.status === "pending" && !file.queued,
      );
      const paths = pending.map((file) => file.path);
      if (paths.length === 0) {
        toast.error("No pending files to process");
        return;
      }
      const state = await invoke<QueueState>("enqueue", {
        payload: {
          files: paths,
          outputDir,
          options,
          processing,
        },
      });
      setFiles((prev) => {
        const used = new Set<string>();
        const newJobs = state.jobs.filter((j) => j.status === "queued");
        return prev.map((file) => {
          if (file.status !== "pending" || file.queued) return file;
          const job = newJobs.find(
            (j) => j.inputPath === file.path && !used.has(j.id),
          );
          if (!job) return file;
          used.add(job.id);
          return { ...file, id: job.id, queued: true };
        });
      });
      await invoke("start_queue");
      addLog("info", `Processing ${paths.length} file(s)...`);
    } catch (err) {
      const message = (err as { message?: string })?.message || String(err);
      toast.error(`Failed to start processing: ${message}`);
      addLog("error", `Start failed: ${message}`);
    }
  };

  const handleStop = useCallback(async () => {
    try {
      await invoke("pause_queue");
      addLog("info", "Queue paused");
    } catch {
      toast.error("Unable to cancel queue");
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return {
    files,
    outputDir,
    setOutputDir,
    showActivity,
    setShowActivity,
    isRunning,
    handleFilesAdded,
    handleFileRemove,
    handleFileReprocess,
    handleClearFiles,
    handleStart,
    handleStop,
  };
}

import { useState, useEffect, useCallback, useMemo } from "react";
import { Toaster, toast } from "sonner";
import { FileDropZone } from "@/components/file-dropzone";
import { OutputDirectory } from "@/components/output-directory";
import { QueueView } from "@/components/queue-view";
import { LogPanel } from "@/components/log-panel";
import { AdvancedOptions, type ProfileValues } from "@/components/advanced-options";
import { Header } from "@/components/header";
import { Button } from "@/components/ui/button";
import { TooltipProvider } from "@/components/ui/tooltip";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { type FileItem, type LogEntry } from "@/types";

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

const mapSettingsToOptions = (values: ProfileValues) => {
  return {
    outputType: values.archiveEnforcement ? "pdfa" : "pdf",
    lossyCompression: values.compression !== "ccitt",
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
  };
};

const mapSettingsToProcessing = (values: ProfileValues) => {
  return {
    maxConcurrentFiles: values.memoryPages,
    tessdataPath: undefined,
    languages: values.languages,
    threadPoolSize: values.cpuCores,
  };
};

const getGreeting = () => {
  const hour = new Date().getHours();
  if (hour < 12) return "Good morning";
  if (hour < 18) return "Good afternoon";
  return "Good evening";
};

export default function App() {
  const [files, setFiles] = useState<FileItem[]>([]);
  const [outputDir, setOutputDir] = useState("");
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [showActivity, setShowActivity] = useState(true);
  const [greeting, setGreeting] = useState(getGreeting);
  const [settings, setSettings] = useState<ProfileValues>({
    cpuCores: Math.max(1, (navigator.hardwareConcurrency || 8) - 2),
    memoryPages: 30,
    binarization: "otsu",
    fixedThreshold: 128,
    deskew: "radon",
    denoiseLevel: 2,
    existingText: "skip",
    psm: "auto",
    compression: "ccitt",
    resolution: "300",
    archiveEnforcement: false,
    languages: "eng",
  });

  const isRunning = useMemo(
    () => files.some((f) => f.status === "processing"),
    [files],
  );

  useEffect(() => {
    const msUntilNext = () => {
      const now = new Date();
      const hour = now.getHours();
      let next = new Date(now);
      if (hour < 12) next.setHours(12, 0, 0, 0);
      else if (hour < 18) next.setHours(18, 0, 0, 0);
      else {
        next.setDate(next.getDate() + 1);
        next.setHours(0, 0, 0, 0);
      }
      return next.getTime() - now.getTime();
    };

    const ref = { current: 0 };
    const schedule = () => {
      ref.current = window.setTimeout(() => {
        setGreeting(getGreeting());
        schedule();
      }, msUntilNext());
    };
    schedule();
    return () => clearTimeout(ref.current);
  }, []);

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

  const addLog = (level: LogEntry["level"], message: string) => {
    setLogs((prev) => [
      ...prev,
      { id: crypto.randomUUID(), timestamp: new Date(), level, message },
    ]);
  };

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
      } catch (err) {
        toast.error("Unable to remove file");
      }
    },
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
  }, []);

  const handleStart = async () => {
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
      const message = typeof err === "string" ? err : String(err);
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
                ? Math.round(
                    (progress.currentPage / progress.totalPages) * 100,
                  )
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
                  status: job.status === "completed" ? "complete" : job.status === "cancelled" ? "paused" : "error",
                  progress: job.status === "completed" ? 100 : file.progress,
                  queued: true,
                }
              : file,
          ),
        );
        addLog(
          job.status === "completed" ? "info" : job.status === "cancelled" ? "warn" : "error",
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
  }, []);

  return (
    <>
      <TooltipProvider>
        <div className="flex gap-6 pt-10 pr-6 pb-6 pl-6 h-dvh overflow-hidden">
          <div className="flex-[3] min-w-0 flex flex-col min-h-0">
            <div className="flex-1 min-h-0 overflow-y-auto">
              <Header
                greeting={greeting}
                showActivity={showActivity}
                onToggleActivity={() => setShowActivity((v) => !v)}
              />
              <div className="space-y-4">
                <FileDropZone onFilesAdded={handleFilesAdded} />
                <OutputDirectory value={outputDir} onChange={setOutputDir} />
                <AdvancedOptions value={settings} onChange={setSettings} />
                <Button className="w-full" size="lg" onClick={handleStart}>
                  {isRunning ? "Add to Queue" : "Start OCR Processing"}
                </Button>
              </div>
            </div>
          </div>

          <div className="flex-[2] min-w-0 flex flex-col gap-2">
            {showActivity ? (
              <>
                <div className="flex-[3] min-h-0">
                  <QueueView
                    files={files}
                    onFileRemove={handleFileRemove}
                    onClear={handleClearFiles}
                    onReprocess={handleFileReprocess}
                    isRunning={isRunning}
                    onStop={handleStop}
                  />
                </div>
                <div className="flex-[1] min-h-0">
                  <LogPanel logs={logs} />
                </div>
              </>
            ) : (
              <div className="flex-1 min-h-0">
                <QueueView
                  files={files}
                  onFileRemove={handleFileRemove}
                  onClear={handleClearFiles}
                  onReprocess={handleFileReprocess}
                  isRunning={isRunning}
                  onStop={handleStop}
                />
              </div>
            )}
          </div>
        </div>
      </TooltipProvider>
      <Toaster
        position="top-right"
        duration={5000}
        toastOptions={{
          classNames: {
            toast:
              "!p-4 !gap-3 !items-center !rounded-xl !shadow-lg !border backdrop-blur-md",
            error:
              "!bg-red-50 dark:!bg-red-950/30 !border-red-600 dark:!border-red-500 !text-red-700 dark:!text-red-400",
            success:
              "!bg-emerald-50 dark:!bg-emerald-950/30 !border-emerald-600 dark:!border-emerald-500 !text-emerald-700 dark:!text-emerald-400",
            warning:
              "!bg-amber-50 dark:!bg-amber-950/30 !border-amber-600 dark:!border-amber-500 !text-amber-700 dark:!text-amber-400",
            info: "!bg-blue-50 dark:!bg-blue-950/30 !border-blue-600 dark:!border-blue-500 !text-blue-700 dark:!text-blue-400",
          },
        }}
      />
    </>
  );
}

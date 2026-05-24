import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useQueue } from "@/hooks/useQueue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

vi.mock("sonner", () => ({
  toast: { error: vi.fn(), success: vi.fn() },
}));

const eventHandlers: Record<string, (event: { payload: unknown }) => void> =
  {};

beforeEach(() => {
  vi.clearAllMocks();
  for (const k of Object.keys(eventHandlers)) delete eventHandlers[k];
  vi.mocked(listen).mockImplementation((event: string, handler: unknown) => {
    eventHandlers[event] = handler as (event: { payload: unknown }) => void;
    return Promise.resolve(() => {});
  });
});

function emitEvent(event: string, payload: unknown) {
  const handler = eventHandlers[event];
  if (handler) handler({ payload });
}

/** Flush pending microtasks so async useEffect setup completes. */
async function flushEffects() {
  await act(async () => {
    await new Promise((r) => setTimeout(r, 0));
  });
}

const defaultSettings = {
  cpuCores: 6,
  memoryPages: 30,
  binarization: "otsu" as const,
  fixedThreshold: 128,
  deskew: "radon" as const,
  denoiseLevel: 2,
  existingText: "skip" as const,
  psm: "auto" as const,
  compression: "ccitt" as const,
  resolution: "300",
  archiveEnforcement: false,
  languages: "eng",
};

describe("useQueue", () => {
  it("handleStart calls invoke('enqueue') with correct payload", async () => {
    vi.mocked(invoke).mockResolvedValue({
      jobs: [{ id: "1", inputPath: "/test.pdf", status: "queued" }],
      isRunning: false,
    });
    const addLog = vi.fn();
    const { result } = renderHook(() => useQueue(addLog));

    act(() => {
      result.current.handleFilesAdded([
        {
          id: "1",
          path: "/test.pdf",
          name: "test.pdf",
          size: 1024,
          status: "pending",
        },
      ]);
    });

    act(() => {
      result.current.setOutputDir("/output");
    });

    await act(async () => {
      await result.current.handleStart(defaultSettings);
    });

    expect(invoke).toHaveBeenCalledWith(
      "enqueue",
      expect.objectContaining({
        payload: expect.objectContaining({
          files: ["/test.pdf"],
          outputDir: "/output",
        }),
      }),
    );
    expect(invoke).toHaveBeenCalledWith("start_queue");
    expect(addLog).toHaveBeenCalledWith("info", "Processing 1 file(s)...");
  });

  it("handleStop calls invoke('pause_queue')", async () => {
    const addLog = vi.fn();
    const { result } = renderHook(() => useQueue(addLog));

    await act(async () => {
      await result.current.handleStop();
    });

    expect(invoke).toHaveBeenCalledWith("pause_queue");
    expect(addLog).toHaveBeenCalledWith("info", "Queue paused");
  });

  it("pipeline-progress event updates file status and progress", async () => {
    const addLog = vi.fn();
    const { result } = renderHook(() => useQueue(addLog));
    await flushEffects();

    act(() => {
      result.current.handleFilesAdded([
        {
          id: "1",
          path: "/test.pdf",
          name: "test.pdf",
          size: 1024,
          status: "pending",
        },
      ]);
    });

    act(() => {
      emitEvent("pipeline-progress", {
        jobId: "1",
        status: "processing",
        currentPage: 3,
        totalPages: 10,
        totalFilesProcessed: 1,
        totalFilesInQueue: 1,
        averageMsPerPage: 500,
      });
    });

    const file = result.current.files.find((f) => f.id === "1");
    expect(file?.status).toBe("processing");
    expect(file?.progress).toBe(30);
  });

  it("pipeline-progress with failed status sets error", async () => {
    const addLog = vi.fn();
    const { result } = renderHook(() => useQueue(addLog));
    await flushEffects();

    act(() => {
      result.current.handleFilesAdded([
        {
          id: "1",
          path: "/test.pdf",
          name: "test.pdf",
          size: 1024,
          status: "pending",
        },
      ]);
    });

    act(() => {
      emitEvent("pipeline-progress", {
        jobId: "1",
        status: "failed",
        currentPage: 0,
        totalPages: 0,
        totalFilesProcessed: 0,
        totalFilesInQueue: 1,
        averageMsPerPage: 0,
        errorMessage: "OCR failed",
      });
    });

    const file = result.current.files.find((f) => f.id === "1");
    expect(file?.status).toBe("error");
    expect(addLog).toHaveBeenCalledWith("error", "OCR failed");
  });

  it("jobFinished event updates file status to complete", async () => {
    const addLog = vi.fn();
    const { result } = renderHook(() => useQueue(addLog));
    await flushEffects();

    act(() => {
      result.current.handleFilesAdded([
        {
          id: "1",
          path: "/test.pdf",
          name: "test.pdf",
          size: 1024,
          status: "pending",
        },
      ]);
    });

    act(() => {
      emitEvent("jobFinished", {
        id: "1",
        inputPath: "/test.pdf",
        outputPath: "/output/test.pdf",
        status: "completed",
      });
    });

    const file = result.current.files.find((f) => f.id === "1");
    expect(file?.status).toBe("complete");
    expect(file?.progress).toBe(100);
    expect(addLog).toHaveBeenCalledWith(
      "info",
      "Completed: /test.pdf → /output/test.pdf",
    );
  });

  it("jobFinished with failed status sets error", async () => {
    const addLog = vi.fn();
    const { result } = renderHook(() => useQueue(addLog));
    await flushEffects();

    act(() => {
      result.current.handleFilesAdded([
        {
          id: "1",
          path: "/test.pdf",
          name: "test.pdf",
          size: 1024,
          status: "pending",
        },
      ]);
    });

    act(() => {
      emitEvent("jobFinished", {
        id: "1",
        inputPath: "/test.pdf",
        outputPath: "/output/test.pdf",
        status: "failed",
        errorMessage: "Out of memory",
      });
    });

    const file = result.current.files.find((f) => f.id === "1");
    expect(file?.status).toBe("error");
    expect(addLog).toHaveBeenCalledWith(
      "error",
      "Failed: Out of memory",
    );
  });

  it("handleClearFiles calls invoke('clear_queue') and clears files", async () => {
    const addLog = vi.fn();
    vi.mocked(invoke).mockResolvedValue({});
    const { result } = renderHook(() => useQueue(addLog));

    act(() => {
      result.current.handleFilesAdded([
        {
          id: "1",
          path: "/test.pdf",
          name: "test.pdf",
          size: 1024,
          status: "pending",
        },
      ]);
    });

    await act(async () => {
      await result.current.handleClearFiles();
    });

    expect(invoke).toHaveBeenCalledWith("clear_queue");
    expect(result.current.files).toHaveLength(0);
    expect(addLog).toHaveBeenCalledWith("info", "Queue cleared");
  });

  it("handleFileRemove calls invoke('remove_job') for queued files", async () => {
    const addLog = vi.fn();
    vi.mocked(invoke).mockResolvedValue({ jobs: [], isRunning: false });
    const { result } = renderHook(() => useQueue(addLog));

    act(() => {
      result.current.handleFilesAdded([
        {
          id: "1",
          path: "/test.pdf",
          name: "test.pdf",
          size: 1024,
          status: "pending",
          queued: true,
        },
      ]);
    });

    await act(async () => {
      await result.current.handleFileRemove("1");
    });

    expect(invoke).toHaveBeenCalledWith("remove_job", { job_id: "1" });
    expect(result.current.files).toHaveLength(0);
    expect(addLog).toHaveBeenCalledWith("info", "Removed: test.pdf");
  });
});

import { emit } from '@tauri-apps/api/event';
import { clearMocks, mockIPC, mockWindows } from '@tauri-apps/api/mocks';
import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest';
import { useQueue } from '@/hooks/useQueue';

vi.mock('@tauri-apps/api/core', async () => ({
  ...(await vi.importActual<typeof import('@tauri-apps/api/core')>(
    '@tauri-apps/api/core',
  )),
}));

vi.mock('@tauri-apps/api/event', async () => ({
  ...(await vi.importActual<typeof import('@tauri-apps/api/event')>(
    '@tauri-apps/api/event',
  )),
}));

beforeAll(() => {
  mockWindows('main');
  Object.defineProperty(globalThis, 'crypto', {
    value: {
      ...((globalThis as any).crypto || {}),
      getRandomValues: (arr: Uint32Array) => {
        for (let i = 0; i < arr.length; i++)
          arr[i] = Math.floor(Math.random() * 2 ** 32);
        return arr;
      },
    },
    configurable: true,
  });
});

afterEach(() => {
  clearMocks();
  // Restore internals after clearMocks so late cleanup listeners
  // (triggered by React passive effect unmount) don't throw.
  const tauriEvent = (window as any).__TAURI_EVENT_PLUGIN_INTERNALS__;
  if (tauriEvent && !tauriEvent.unregisterListener) {
    tauriEvent.unregisterListener = () => {};
  }
  const tauriInternals = (window as any).__TAURI_INTERNALS__;
  if (tauriInternals && !tauriInternals.invoke) {
    tauriInternals.invoke = async () => {};
  }
});

function makeProfileSettings() {
  return {
    languages: ['eng'],
    deskew: 'radon' as const,
    denoiseLevel: 1,
    existingText: 'skip' as const,
    psm: 'auto' as const,
    compression: 'flate' as const,
    resolution: '300',
    memoryPages: 2,
    archiveEnforcement: false,
    safeMode: false,
    binarization: 'otsu' as const,
    fixedThreshold: 128,
    outputType: 'pdf' as const,
    continueOnError: true,
  };
}

describe('App e2e — sample file queue lifecycle via mockIPC', () => {
  it('processes poster.pdf through mockIPC with real invoke/emit flow', async () => {
    const posterPath = '/samples/poster.pdf';
    const outputPath = '/output/poster_ocr.pdf';

    mockIPC(
      (cmd, args) => {
        switch (cmd) {
          case 'check_file_encrypted':
            return { encrypted: false, fileId: (args as any)?.path };
          case 'ensure_language_packs':
            return { downloaded: [], skipped: ['eng'], errors: {} };
          case 'get_history':
            return [];
          case 'enqueue':
            return {
              jobs: [
                {
                  id: 'job-0',
                  inputPath: posterPath,
                  outputPath,
                  status: 'queued',
                  percent: 0,
                },
              ],
              isRunning: true,
            };
          case 'start_queue':
            return undefined;
          default:
            return undefined;
        }
      },
      { shouldMockEvents: true },
    );

    const addLog = vi.fn();
    const { result } = renderHook(() => useQueue(addLog));

    // Step 1: Add files to the queue.
    await act(async () => {
      await result.current.handleFilesAdded([
        {
          id: 'file-1',
          path: posterPath,
          name: 'poster.pdf',
          size: 695000,
          status: 'pending',
        },
      ]);
    });

    expect(result.current.files).toHaveLength(1);
    expect(result.current.files[0].name).toBe('poster.pdf');

    // Step 2: Set output directory and start queue.
    await act(async () => {
      result.current.setOutputDir('/output');
    });

    await act(async () => {
      await result.current.handleStart(makeProfileSettings());
    });

    expect(addLog).toHaveBeenCalledWith('info', 'Processing 1 file(s)...');

    // Step 3: Simulate progress events via real emit().
    await act(async () => {
      await emit('jobProgress', {
        id: 'job-0',
        inputPath: posterPath,
        outputPath,
        status: 'running',
        percent: 30,
      });
    });

    await act(async () => {
      await emit('pipeline-progress', {
        jobId: 'job-0',
        status: 'processing',
        currentPage: 3,
        totalPages: 5,
        totalFilesProcessed: 0,
        totalFilesInQueue: 1,
        averageMsPerPage: 0,
        errorMessage: null,
      });
    });

    // Step 4: Emit job completion. Assert BEFORE queueState clears the file.
    await act(async () => {
      await emit('jobFinished', {
        id: 'job-0',
        inputPath: posterPath,
        outputPath,
        status: 'completed',
        percent: 100,
      });
    });

    // The file should be marked complete before queueState removes it.
    expect(result.current.files).toHaveLength(1);
    expect(result.current.files[0].status).toBe('complete');

    expect(addLog).toHaveBeenCalledWith(
      'info',
      `Completed: ${posterPath} → ${outputPath}`,
    );

    // Step 5: queueState with empty jobs should mark isRunning false.
    await act(async () => {
      await emit('queueState', {
        jobs: [],
        isRunning: false,
      });
    });

    expect(result.current.isRunning).toBe(false);
  });

  it('handles a failed job gracefully via mockIPC', async () => {
    const filePath = '/samples/skew.pdf';

    mockIPC(
      (cmd, args) => {
        switch (cmd) {
          case 'check_file_encrypted':
            return { encrypted: false, fileId: (args as any)?.path };
          case 'ensure_language_packs':
            return { downloaded: [], skipped: ['eng'], errors: {} };
          case 'get_history':
            return [];
          case 'enqueue':
            return {
              jobs: [
                {
                  id: 'job-1',
                  inputPath: filePath,
                  outputPath: '/output/skew_ocr.pdf',
                  status: 'queued',
                  percent: 0,
                },
              ],
              isRunning: true,
            };
          case 'start_queue':
            return undefined;
          default:
            return undefined;
        }
      },
      { shouldMockEvents: true },
    );

    const addLog = vi.fn();
    const { result } = renderHook(() => useQueue(addLog));

    await act(async () => {
      await result.current.handleFilesAdded([
        {
          id: 'file-2',
          path: filePath,
          name: 'skew.pdf',
          size: 76000,
          status: 'pending',
        },
      ]);
    });

    await act(async () => {
      result.current.setOutputDir('/output');
    });

    await act(async () => {
      await result.current.handleStart(makeProfileSettings());
    });

    // Simulate failure event.
    await act(async () => {
      await emit('jobFinished', {
        id: 'job-1',
        inputPath: filePath,
        outputPath: '',
        status: 'failed',
        errorMessage: 'Out of memory',
      });
    });

    expect(result.current.files).toHaveLength(1);
    expect(result.current.files[0].status).toBe('error');

    expect(addLog).toHaveBeenCalledWith('error', 'Failed: Out of memory');
  });
});

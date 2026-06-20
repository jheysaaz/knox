import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { act, renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useEventListener } from '@/hooks/useEventListener';
import type { FileItem } from '@/types';

const eventHandlers: Record<string, (event: { payload: unknown }) => void> = {};

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

describe('useEventListener', () => {
  const addLog = vi.fn();
  const setFiles = vi.fn() as unknown as React.Dispatch<
    React.SetStateAction<FileItem[]>
  >;

  beforeEach(() => {
    vi.mocked(invoke).mockResolvedValue([]);
  });

  it('returns initial state', () => {
    const { result } = renderHook(() => useEventListener(setFiles, addLog));
    expect(result.current.history).toEqual([]);
    expect(result.current.isRunning).toBe(false);
  });

  it('ensureListeners registers all event handlers', async () => {
    const { result } = renderHook(() => useEventListener(setFiles, addLog));
    await act(async () => {
      await result.current.ensureListeners();
    });
    expect(listen).toHaveBeenCalledWith(
      'pipeline-progress',
      expect.any(Function),
    );
    expect(listen).toHaveBeenCalledWith('queueState', expect.any(Function));
    expect(listen).toHaveBeenCalledWith('jobFinished', expect.any(Function));
    expect(listen).toHaveBeenCalledWith('jobProgress', expect.any(Function));
    expect(listen).toHaveBeenCalledWith('historyUpdated', expect.any(Function));
  });

  it('does not re-register listeners on second call', async () => {
    const { result } = renderHook(() => useEventListener(setFiles, addLog));
    await act(async () => {
      await result.current.ensureListeners();
    });
    await act(async () => {
      await result.current.ensureListeners();
    });
    expect(listen).toHaveBeenCalledTimes(5);
  });

  it('loads history on listener initialization', async () => {
    const mockHistory = [
      {
        id: '1',
        inputPath: '/a.pdf',
        outputPath: '/a.pdf',
        status: 'completed',
        startedAt: 1000,
        finishedAt: 2000,
        durationMs: 1000,
      },
    ];
    vi.mocked(invoke).mockResolvedValue(mockHistory);
    const { result } = renderHook(() => useEventListener(setFiles, addLog));
    await act(async () => {
      await result.current.ensureListeners();
    });
    expect(result.current.history).toEqual(mockHistory);
  });

  it('handles pipeline-progress event', async () => {
    const { result } = renderHook(() => useEventListener(setFiles, addLog));
    await act(async () => {
      await result.current.ensureListeners();
    });
    act(() => {
      emitEvent('pipeline-progress', {
        jobId: 'job-1',
        status: 'ocr',
        currentPage: 2,
        totalPages: 5,
        totalFilesProcessed: 0,
        totalFilesInQueue: 1,
        averageMsPerPage: 300,
      });
    });
    expect(setFiles).toHaveBeenCalled();
  });

  it('handles pipeline-progress failure with error message', async () => {
    const { result } = renderHook(() => useEventListener(setFiles, addLog));
    await act(async () => {
      await result.current.ensureListeners();
    });
    act(() => {
      emitEvent('pipeline-progress', {
        jobId: 'job-1',
        status: 'failed',
        currentPage: 1,
        totalPages: 1,
        totalFilesProcessed: 0,
        totalFilesInQueue: 1,
        averageMsPerPage: 0,
        errorMessage: 'OCR error',
      });
    });
    expect(addLog).toHaveBeenCalledWith('error', 'OCR error');
  });

  it('handles queueState event', async () => {
    const { result } = renderHook(() => useEventListener(setFiles, addLog));
    await act(async () => {
      await result.current.ensureListeners();
    });
    act(() => {
      emitEvent('queueState', {
        jobs: [],
        isRunning: true,
      });
    });
    expect(result.current.isRunning).toBe(true);
  });

  it('handles jobFinished event', async () => {
    const { result } = renderHook(() => useEventListener(setFiles, addLog));
    await act(async () => {
      await result.current.ensureListeners();
    });
    act(() => {
      emitEvent('jobFinished', {
        id: 'job-1',
        inputPath: '/in.pdf',
        outputPath: '/out.pdf',
        status: 'completed',
        percent: 100,
        options: {},
      });
    });
    expect(addLog).toHaveBeenCalledWith(
      'info',
      expect.stringContaining('Completed'),
    );
  });

  it('handles jobFinished with cancelled status', async () => {
    const { result } = renderHook(() => useEventListener(setFiles, addLog));
    await act(async () => {
      await result.current.ensureListeners();
    });
    act(() => {
      emitEvent('jobFinished', {
        id: 'job-1',
        inputPath: '/in.pdf',
        outputPath: '/out.pdf',
        status: 'cancelled',
        percent: 50,
        options: {},
      });
    });
    expect(addLog).toHaveBeenCalledWith(
      'warn',
      expect.stringContaining('Paused'),
    );
  });

  it('handles jobFinished with failed status', async () => {
    const { result } = renderHook(() => useEventListener(setFiles, addLog));
    await act(async () => {
      await result.current.ensureListeners();
    });
    act(() => {
      emitEvent('jobFinished', {
        id: 'job-1',
        inputPath: '/in.pdf',
        outputPath: '/out.pdf',
        status: 'failed',
        percent: 30,
        errorMessage: 'timeout',
        options: {},
      });
    });
    expect(addLog).toHaveBeenCalledWith(
      'error',
      expect.stringContaining('Failed'),
    );
  });

  it('calls onJobError when a job fails', async () => {
    const onJobError = vi.fn();
    const { result } = renderHook(() =>
      useEventListener(setFiles, addLog, undefined, onJobError),
    );
    await act(async () => {
      await result.current.ensureListeners();
    });
    act(() => {
      emitEvent('jobFinished', {
        id: 'job-1',
        inputPath: '/in.pdf',
        outputPath: '/out.pdf',
        status: 'failed',
        percent: 30,
        errorMessage: 'timeout',
        options: {},
      });
    });
    expect(onJobError).toHaveBeenCalledWith('/in.pdf', 'timeout');
  });

  it('does not call onJobError for completed jobs', async () => {
    const onJobError = vi.fn();
    const { result } = renderHook(() =>
      useEventListener(setFiles, addLog, undefined, onJobError),
    );
    await act(async () => {
      await result.current.ensureListeners();
    });
    act(() => {
      emitEvent('jobFinished', {
        id: 'job-1',
        inputPath: '/in.pdf',
        outputPath: '/out.pdf',
        status: 'completed',
        percent: 100,
        options: {},
      });
    });
    expect(onJobError).not.toHaveBeenCalled();
  });

  it('handles historyUpdated event', async () => {
    const { result } = renderHook(() => useEventListener(setFiles, addLog));
    await act(async () => {
      await result.current.ensureListeners();
    });
    const newHistory = [{ id: 'h1' }];
    act(() => {
      emitEvent('historyUpdated', newHistory);
    });
    expect(result.current.history).toEqual(newHistory);
  });

  it('clearHistory clears history', async () => {
    const { result } = renderHook(() => useEventListener(setFiles, addLog));
    await act(async () => {
      await result.current.ensureListeners();
    });
    act(() => {
      emitEvent('historyUpdated', [{ id: 'h1' }]);
    });
    expect(result.current.history).toHaveLength(1);
    await act(async () => {
      await result.current.handleClearHistory();
    });
    expect(invoke).toHaveBeenCalledWith('clear_history');
  });

  it('cleans up listeners on unmount', async () => {
    const unlisten = vi.fn();
    vi.mocked(listen).mockResolvedValue(unlisten);
    const { result, unmount } = renderHook(() =>
      useEventListener(setFiles, addLog),
    );
    await act(async () => {
      await result.current.ensureListeners();
    });
    unmount();
    expect(unlisten).toHaveBeenCalledTimes(5);
  });
});

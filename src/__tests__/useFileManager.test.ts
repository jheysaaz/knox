import { invoke } from '@tauri-apps/api/core';
import { act, renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useFileManager } from '@/hooks/useFileManager';
import type { FileItem } from '@/types';

describe('useFileManager', () => {
  const addLog = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('starts with empty files', () => {
    const { result } = renderHook(() => useFileManager(addLog));
    expect(result.current.files).toEqual([]);
  });

  it('adds files via handleFilesAdded', () => {
    const { result } = renderHook(() => useFileManager(addLog));
    const newFiles: FileItem[] = [
      {
        id: '1',
        path: '/a.pdf',
        name: 'a.pdf',
        size: 100,
        status: 'pending',
      },
      {
        id: '2',
        path: '/b.pdf',
        name: 'b.pdf',
        size: 200,
        status: 'pending',
      },
    ];
    act(() => {
      result.current.handleFilesAdded(newFiles);
    });
    expect(result.current.files).toHaveLength(2);
    expect(result.current.files[0].queued).toBe(false);
  });

  it('removes a non-queued file locally', () => {
    const { result } = renderHook(() => useFileManager(addLog));
    act(() => {
      result.current.handleFilesAdded([
        {
          id: '1',
          path: '/a.pdf',
          name: 'a.pdf',
          size: 100,
          status: 'pending',
        },
      ]);
    });
    act(() => {
      result.current.handleFileRemove('1');
    });
    expect(result.current.files).toHaveLength(0);
    expect(addLog).toHaveBeenCalledWith('info', 'Removed: a.pdf');
  });

  it('calls remove_job for queued files', async () => {
    vi.mocked(invoke).mockResolvedValue({});
    const { result } = renderHook(() => useFileManager(addLog));
    act(() => {
      result.current.handleFilesAdded([
        {
          id: '1',
          path: '/a.pdf',
          name: 'a.pdf',
          size: 100,
          status: 'pending',
          queued: true,
        },
      ]);
    });
    await act(async () => {
      await result.current.handleFileRemove('1');
    });
    expect(invoke).toHaveBeenCalledWith('remove_job', { job_id: '1' });
  });

  it('reprocess resets file to pending', async () => {
    const { result } = renderHook(() => useFileManager(addLog));
    act(() => {
      result.current.handleFilesAdded([
        {
          id: '1',
          path: '/a.pdf',
          name: 'a.pdf',
          size: 100,
          status: 'complete',
          queued: true,
        },
      ]);
    });
    await act(async () => {
      await result.current.handleFileReprocess('1');
    });
    const file = result.current.files[0];
    expect(file.status).toBe('pending');
    expect(file.queued).toBe(false);
    expect(file.progress).toBeUndefined();
  });

  it('clearFiles calls clear_queue and empties files', async () => {
    vi.mocked(invoke).mockResolvedValue({});
    const { result } = renderHook(() => useFileManager(addLog));
    act(() => {
      result.current.handleFilesAdded([
        {
          id: '1',
          path: '/a.pdf',
          name: 'a.pdf',
          size: 100,
          status: 'pending',
        },
      ]);
    });
    await act(async () => {
      await result.current.handleClearFiles();
    });
    expect(invoke).toHaveBeenCalledWith('clear_queue');
    expect(result.current.files).toHaveLength(0);
    expect(addLog).toHaveBeenCalledWith('info', 'Queue cleared');
  });

  it('prevents removal of processing file', () => {
    const { result } = renderHook(() => useFileManager(addLog));
    act(() => {
      result.current.handleFilesAdded([
        {
          id: '1',
          path: '/a.pdf',
          name: 'a.pdf',
          size: 100,
          status: 'processing',
        },
      ]);
    });
    act(() => {
      result.current.handleFileRemove('1');
    });
    // File should still be present since removal was blocked
    expect(result.current.files).toHaveLength(1);
  });
});

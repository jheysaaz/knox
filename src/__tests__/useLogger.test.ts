import { act, renderHook } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { useLogger } from '@/hooks/useLogger';

describe('useLogger', () => {
  it('starts with empty logs', () => {
    const { result } = renderHook(() => useLogger());
    expect(result.current.logs).toEqual([]);
  });

  it('addLog creates a log entry', () => {
    const { result } = renderHook(() => useLogger());
    act(() => {
      result.current.addLog('info', 'test message');
    });
    expect(result.current.logs).toHaveLength(1);
    expect(result.current.logs[0].level).toBe('info');
    expect(result.current.logs[0].message).toBe('test message');
    expect(result.current.logs[0].id).toBeDefined();
    expect(result.current.logs[0].timestamp).toBeInstanceOf(Date);
  });

  it('addLog creates entries with different levels', () => {
    const { result } = renderHook(() => useLogger());
    act(() => {
      result.current.addLog('info', 'info msg');
      result.current.addLog('warn', 'warn msg');
      result.current.addLog('error', 'error msg');
    });
    expect(result.current.logs).toHaveLength(3);
    expect(result.current.logs[0].level).toBe('info');
    expect(result.current.logs[1].level).toBe('warn');
    expect(result.current.logs[2].level).toBe('error');
  });

  it('truncates logs at MAX_LOG_ENTRIES (500)', () => {
    const { result } = renderHook(() => useLogger());
    act(() => {
      for (let i = 0; i < 510; i++) {
        result.current.addLog('info', `msg ${i}`);
      }
    });
    expect(result.current.logs).toHaveLength(500);
    expect(result.current.logs[0].message).toBe('msg 10');
    expect(result.current.logs[499].message).toBe('msg 509');
  });

  it('does not truncate below limit', () => {
    const { result } = renderHook(() => useLogger());
    act(() => {
      for (let i = 0; i < 100; i++) {
        result.current.addLog('info', `msg ${i}`);
      }
    });
    expect(result.current.logs).toHaveLength(100);
  });
});

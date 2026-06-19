import { invoke } from '@tauri-apps/api/core';
import { save } from '@tauri-apps/plugin-dialog';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { LogPanel } from '@/components/log-panel';
import type { LogEntry } from '@/types';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({
  save: vi.fn(),
}));

const makeLog = (overrides: Partial<LogEntry> = {}): LogEntry => ({
  id: '1',
  timestamp: new Date('2025-01-01T12:00:00'),
  level: 'info',
  message: 'test message',
  ...overrides,
});

describe('LogPanel', () => {
  it('shows empty state', () => {
    render(<LogPanel logs={[]} />);
    expect(screen.getByText('No activity yet')).toBeInTheDocument();
  });

  it('renders log entries', () => {
    render(<LogPanel logs={[makeLog({ message: 'hello' })]} />);
    expect(screen.getByText('hello')).toBeInTheDocument();
  });

  it('renders severity label for each log', () => {
    const logs: LogEntry[] = [
      makeLog({ id: '1', level: 'info', message: 'info msg' }),
      makeLog({ id: '2', level: 'warn', message: 'warn msg' }),
      makeLog({ id: '3', level: 'error', message: 'error msg' }),
    ];
    render(<LogPanel logs={logs} />);
    expect(screen.getByText('[INFO]')).toBeInTheDocument();
    expect(screen.getByText('[WARN]')).toBeInTheDocument();
    expect(screen.getByText('[ERROR]')).toBeInTheDocument();
  });

  it('saves logs to file', async () => {
    vi.mocked(save).mockResolvedValue('/logs/session.log');
    render(
      <LogPanel
        logs={[
          makeLog({ level: 'info', message: 'started' }),
          makeLog({ level: 'warn', message: 'warning' }),
        ]}
      />,
    );
    const user = userEvent.setup();
    await user.click(screen.getByTitle('Save logs'));
    expect(invoke).toHaveBeenCalledWith('write_log_file', {
      path: '/logs/session.log',
      content: expect.stringContaining('started'),
    });
  });
});

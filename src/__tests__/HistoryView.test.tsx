import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { HistoryView } from '@/components/history-view';
import type { HistoryEntry } from '@/types';

const makeEntry = (overrides: Partial<HistoryEntry> = {}): HistoryEntry => ({
  id: '1',
  inputPath: '/docs/report.pdf',
  outputPath: '/output/report.pdf',
  status: 'completed',
  startedAt: 1700000000,
  finishedAt: 1700000100,
  durationMs: 100000,
  options: {
    outputType: 'pdf',
    safeMode: false,
    maxConcurrency: null,
    binarization: 'otsu',
    fixedThreshold: 128,
    deskewMode: 'disabled',
    denoiseLevel: 0,
    existingText: 'skip',
    psm: 'auto',
    compression: 'ccitt',
    resolutionDpi: 300,
    archiveEnforcement: false,
    languages: 'eng',
    memoryPages: null,
    continueOnError: false,
    password: null,
  },
  ...overrides,
});

describe('HistoryView', () => {
  it('shows empty state', () => {
    render(<HistoryView entries={[]} />);
    expect(screen.getByText('No history yet')).toBeInTheDocument();
  });

  it('renders completed entry', () => {
    render(<HistoryView entries={[makeEntry()]} />);
    expect(screen.getByText('report.pdf')).toBeInTheDocument();
    expect(screen.getByText('Completed')).toBeInTheDocument();
  });

  it('renders failed entry', () => {
    render(<HistoryView entries={[makeEntry({ status: 'failed' })]} />);
    expect(screen.getByText('Failed')).toBeInTheDocument();
  });

  it('renders cancelled entry', () => {
    render(<HistoryView entries={[makeEntry({ status: 'cancelled' })]} />);
    expect(screen.getByText('Cancelled')).toBeInTheDocument();
  });

  it('formats duration in seconds', () => {
    render(<HistoryView entries={[makeEntry({ durationMs: 2500 })]} />);
    expect(screen.getByText('2.5s')).toBeInTheDocument();
  });

  it('formats duration in minutes', () => {
    render(<HistoryView entries={[makeEntry({ durationMs: 125000 })]} />);
    expect(screen.getByText('2m 5s')).toBeInTheDocument();
  });
});

import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { QueueView } from '@/components/queue-view';
import type { FileItem } from '@/types';

const makeFile = (overrides: Partial<FileItem> = {}): FileItem => ({
  id: '1',
  path: '/path/to/doc.pdf',
  name: 'doc.pdf',
  size: 1024,
  status: 'pending',
  ...overrides,
});

describe('QueueView', () => {
  const onFileRemove = vi.fn();

  it('shows empty state', () => {
    render(<QueueView files={[]} onFileRemove={onFileRemove} />);
    expect(screen.getByText('No files added yet')).toBeInTheDocument();
  });

  it('renders file list', () => {
    render(
      <QueueView
        files={[makeFile({ name: 'test.pdf' })]}
        onFileRemove={onFileRemove}
      />,
    );
    expect(screen.getByText('test.pdf')).toBeInTheDocument();
  });

  it('shows status labels for each state', () => {
    const files: FileItem[] = [
      makeFile({ id: '1', status: 'pending' }),
      makeFile({ id: '2', status: 'processing', progress: 50 }),
      makeFile({ id: '3', status: 'complete' }),
      makeFile({ id: '4', status: 'error' }),
      makeFile({ id: '5', status: 'paused' }),
    ];
    render(
      <QueueView files={files} onFileRemove={onFileRemove} isRunning={true} />,
    );
    expect(screen.getByText('Pending')).toBeInTheDocument();
    expect(screen.getByText('Processing...')).toBeInTheDocument();
    expect(screen.getByText('Complete')).toBeInTheDocument();
    expect(screen.getByText('Error')).toBeInTheDocument();
    expect(screen.getByText('Paused')).toBeInTheDocument();
  });

  it('calls onFileRemove when remove button clicked', async () => {
    render(
      <QueueView
        files={[makeFile({ id: '1', name: 'doc.pdf' })]}
        onFileRemove={onFileRemove}
      />,
    );
    const user = userEvent.setup();
    const removeButtons = screen.getAllByRole('button');
    const closeBtn = removeButtons[removeButtons.length - 1];
    await user.click(closeBtn);
    expect(onFileRemove).toHaveBeenCalledWith('1');
  });

  it('shows progress bar for processing files', () => {
    render(
      <QueueView
        files={[makeFile({ id: '1', status: 'processing', progress: 50 })]}
        onFileRemove={onFileRemove}
        isRunning={true}
      />,
    );
    const progressBar = document.querySelector('[role="progressbar"]');
    expect(progressBar).toBeInTheDocument();
  });
});

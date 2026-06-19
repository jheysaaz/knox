import { open } from '@tauri-apps/plugin-dialog';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { OutputDirectory } from '@/components/output-directory';

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
}));

describe('OutputDirectory', () => {
  const onChange = vi.fn();

  it('renders label and input', () => {
    render(<OutputDirectory value="" onChange={onChange} />);
    expect(screen.getByText('Output Directory')).toBeInTheDocument();
    expect(
      screen.getByPlaceholderText('Select output directory...'),
    ).toBeInTheDocument();
  });

  it('opens directory dialog on browse click', async () => {
    vi.mocked(open).mockResolvedValue('/output/path');
    render(<OutputDirectory value="" onChange={onChange} />);
    const user = userEvent.setup();
    await user.click(screen.getByTitle('Browse'));
    expect(open).toHaveBeenCalledWith({
      directory: true,
      title: 'Select Output Directory',
    });
    expect(onChange).toHaveBeenCalledWith('/output/path');
  });

  it('updates value on manual input', async () => {
    render(<OutputDirectory value="" onChange={onChange} />);
    const user = userEvent.setup();
    const input = screen.getByPlaceholderText('Select output directory...');
    await user.type(input, '/manual/path');
    expect(onChange).toHaveBeenCalled();
  });

  it('shows the current value', () => {
    render(<OutputDirectory value="/current/path" onChange={onChange} />);
    expect(screen.getByDisplayValue('/current/path')).toBeInTheDocument();
  });
});

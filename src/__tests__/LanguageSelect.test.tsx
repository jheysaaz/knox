import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { LanguageSelect } from '@/components/language-select';

describe('LanguageSelect', () => {
  const onChange = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('shows placeholder when no languages selected', () => {
    render(<LanguageSelect value={[]} onChange={onChange} />);
    expect(screen.getByText('Select languages')).toBeInTheDocument();
  });

  it('shows count when languages selected', () => {
    render(<LanguageSelect value={['eng', 'spa']} onChange={onChange} />);
    expect(screen.getByText('2 selected')).toBeInTheDocument();
  });

  it('opens dropdown on click', async () => {
    const user = userEvent.setup();
    render(<LanguageSelect value={[]} onChange={onChange} />);
    await user.click(screen.getByText('Select languages'));
    expect(screen.getByText('English')).toBeInTheDocument();
    expect(screen.getByText('Spanish')).toBeInTheDocument();
  });

  it('filters languages by search', async () => {
    const user = userEvent.setup();
    render(<LanguageSelect value={[]} onChange={onChange} />);
    await user.click(screen.getByText('Select languages'));
    const searchInput = screen.getByPlaceholderText('Search languages...');
    await user.type(searchInput, 'fren');
    expect(screen.getByText('French')).toBeInTheDocument();
    expect(screen.queryByText('English')).not.toBeInTheDocument();
  });

  it('shows no results message for bad search', async () => {
    const user = userEvent.setup();
    render(<LanguageSelect value={[]} onChange={onChange} />);
    await user.click(screen.getByText('Select languages'));
    const searchInput = screen.getByPlaceholderText('Search languages...');
    await user.type(searchInput, 'zzzzz');
    expect(screen.getByText('No languages match')).toBeInTheDocument();
  });

  it('toggles a language on click', async () => {
    const user = userEvent.setup();
    render(<LanguageSelect value={[]} onChange={onChange} />);
    await user.click(screen.getByText('Select languages'));
    await user.click(screen.getByText('English'));
    expect(onChange).toHaveBeenCalledWith(['eng']);
  });

  it('removes a language on deselect', async () => {
    const user = userEvent.setup();
    render(<LanguageSelect value={['eng', 'spa']} onChange={onChange} />);
    await user.click(screen.getByText('2 selected'));
    const buttons = screen.getAllByText('English');
    const inDropdown = buttons[buttons.length - 1];
    await user.click(inDropdown);
    expect(onChange).toHaveBeenCalledWith(['spa']);
  });

  it('closes dropdown when clicking outside', async () => {
    const user = userEvent.setup();
    render(
      <div>
        <span data-testid="outside">outside</span>
        <LanguageSelect value={[]} onChange={onChange} />
      </div>,
    );
    await user.click(screen.getByText('Select languages'));
    expect(screen.getByText('English')).toBeInTheDocument();
    await user.click(screen.getByTestId('outside'));
    expect(screen.queryByText('English')).not.toBeInTheDocument();
  });

  it('shows selected language chips with remove buttons', () => {
    render(<LanguageSelect value={['eng', 'spa']} onChange={onChange} />);
    expect(screen.getByText('English')).toBeInTheDocument();
    expect(screen.getByText('Spanish')).toBeInTheDocument();
  });

  it('removes language via chip X button', async () => {
    const user = userEvent.setup();
    render(<LanguageSelect value={['eng']} onChange={onChange} />);
    const chips = screen.getAllByText('English');
    const chip = chips[0];
    const xButton = chip.closest('span')?.querySelector('button');
    expect(xButton).toBeTruthy();
    if (xButton) await user.click(xButton);
    expect(onChange).toHaveBeenCalledWith([]);
  });
});

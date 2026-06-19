import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { Header } from '@/components/header';

describe('Header', () => {
  it('renders greeting', () => {
    render(<Header greeting="Good morning" />);
    expect(screen.getByText('Good morning')).toBeInTheDocument();
  });

  it('renders theme toggle button', () => {
    render(<Header greeting="Hello" />);
    expect(screen.getByTitle('Toggle theme')).toBeInTheDocument();
  });

  it('toggles theme on button click', async () => {
    const user = userEvent.setup();
    render(<Header greeting="Hello" />);
    await user.click(screen.getByTitle('Toggle theme'));
    expect(document.documentElement.classList.contains('dark')).toBe(true);
  });
});

import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { ProfileValues } from '@/components/advanced-options';
import { AdvancedOptions } from '@/components/advanced-options';
import { TooltipProvider } from '@/components/ui/tooltip';

const defaultValues: ProfileValues = {
  memoryPages: 30,
  binarization: 'otsu',
  fixedThreshold: 128,
  deskew: 'radon',
  denoiseLevel: 2,
  existingText: 'skip',
  psm: 'auto',
  compression: 'ccitt',
  resolution: '300',
  archiveEnforcement: false,
  languages: ['eng'],
  safeMode: false,
};

const renderWithTooltip = (ui: React.ReactElement) =>
  render(<TooltipProvider>{ui}</TooltipProvider>);

describe('AdvancedOptions', () => {
  const onChange = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders profile tabs', () => {
    renderWithTooltip(
      <AdvancedOptions value={defaultValues} onChange={onChange} />,
    );
    expect(screen.getByText('Balanced')).toBeInTheDocument();
    expect(screen.getByText('Max Compression')).toBeInTheDocument();
    expect(screen.getByText('High Fidelity')).toBeInTheDocument();
    expect(screen.getByText('Custom')).toBeInTheDocument();
  });

  it('shows settings panel when on Custom tab', () => {
    // Using modified values forces Custom tab detection
    renderWithTooltip(
      <AdvancedOptions value={defaultValues} onChange={onChange} />,
    );
    // Click the Custom tab to activate it
    const customTab = screen.getByText('Custom');
    customTab.click();
    expect(screen.getByText('In-Memory Page Cap')).toBeInTheDocument();
  });

  it('shows fixed threshold slider when binarization is fixed', async () => {
    const fixedValues = { ...defaultValues, binarization: 'fixed' as const };
    renderWithTooltip(
      <AdvancedOptions value={fixedValues} onChange={onChange} />,
    );
    // Click Custom tab to show settings
    await userEvent.setup().click(screen.getByText('Custom'));
    const fixedThresholdElements = screen.getAllByText('Fixed Threshold');
    expect(fixedThresholdElements.length).toBeGreaterThanOrEqual(2);
  });

  it('hides fixed threshold slider for non-fixed binarization', async () => {
    renderWithTooltip(
      <AdvancedOptions value={defaultValues} onChange={onChange} />,
    );
    await userEvent.setup().click(screen.getByText('Custom'));
    expect(screen.queryByText('Fixed Threshold')).not.toBeInTheDocument();
  });

  it('calls onChange when profile tab is clicked', async () => {
    renderWithTooltip(
      <AdvancedOptions value={defaultValues} onChange={onChange} />,
    );
    const user = userEvent.setup();
    await user.click(screen.getByText('High Fidelity'));
    expect(onChange).toHaveBeenCalled();
  });
});

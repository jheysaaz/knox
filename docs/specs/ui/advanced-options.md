# AdvancedOptions — OCR Settings Profile Component

## User Journey
1. User sees 4 profile tabs: Balanced, Max Compression, High Fidelity, Custom
2. Selecting a preset tab applies predefined settings
3. Selecting Custom allows full control via SettingsPanel
4. SettingsPanel has 4 sections: Hardware, Pre-processing, OCR Engine, Compression

## Props
- `value: ProfileValues` — current settings
- `onChange: (next: ProfileValues) => void` — called when settings change

## Profile Presets
- **Balanced**: cpuCores=max-2, memoryPages=30, otsu, radon, ccitt, 300dpi
- **Max Compression**: cpuCores=max, memoryPages=15, otsu, radon, ccitt, 150dpi, denoise=4
- **High Fidelity**: cpuCores=floor(max/2), memoryPages=50, fixed, radon, flate, 600dpi
- **Custom**: current user-modified values

## SettingsPanel Controls
- **Hardware**: Thread Pool Capacity (1-16 slider), In-Memory Page Cap (5-100 slider)
- **Image Pre-processing**: Thresholding Mode (select), Fixed Threshold (0-255 slider, conditional), Orientation Alignment (select), Despeckle Intensity (0-5 slider)
- **OCR**: Ingestion Override (radio), PSM (select), Languages (input)
- **Compression**: Bi-level Stream Compression (select), DPI (select), PDF/A Compliance (switch)

## Acceptance Criteria
- Profile tabs render with icons and labels
- Selecting preset calls onChange with preset values
- SettingsPanel renders only on Custom tab
- Fixed threshold slider only shows when binarization="fixed"
- All controls call onChange with correct patch values
- Custom tab persists when user modifies any control

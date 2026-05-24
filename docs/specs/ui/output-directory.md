# OutputDirectory — Output Directory Picker

## User Journey
1. User sees a labeled field with an input and browse button
2. User can type a path manually
3. User can click browse to open a directory picker dialog
4. Selected path is displayed in the input

## Props
- `value: string` — current directory path
- `onChange: (value: string) => void` — called on selection or manual input

## Behavior
- Browse button opens `open({ directory: true })` dialog
- Manual input calls `onChange` on each keystroke
- Displays current value in the input field

## States
- **Empty**: Placeholder "Select output directory..."
- **Filled**: Shows selected path

## Acceptance Criteria
- Browse button opens directory dialog
- Directory selection updates value
- Manual input updates value
- Current value is displayed in input

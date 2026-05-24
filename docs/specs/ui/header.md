# Header — App Header Component

## User Journey
1. User sees a greeting ("Good morning/afternoon/evening" based on time)
2. User can toggle dark/light theme via sun/moon button
3. User can show/hide the activity panel via bug icon button

## Props
- `greeting: string` — time-based greeting text
- `showActivity: boolean` — whether activity panel is visible
- `onToggleActivity: () => void` — toggle activity panel

## Behavior
- Greeting updates at 12:00 and 18:00
- Theme toggle persists to `localStorage` (key: "theme")
- Dark mode adds `.dark` class to `<html>`
- Initial theme reads from localStorage, falls back to `prefers-color-scheme`

## Acceptance Criteria
- Greeting text is displayed
- Theme toggle switches dark/light and persists
- Bug button shows "Hide activity" or "Show activity" based on state
- Settings button renders (placeholder)

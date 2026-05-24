# Frontend Test Strategy

## Framework
- **Runner**: Vitest 4
- **Environment**: jsdom
- **Library**: @testing-library/react + @testing-library/jest-dom + @testing-library/user-event
- **Mocks**: Module-level mocks for all Tauri APIs

## Mock Strategy
All Tauri modules are mocked at the module level:
- `@tauri-apps/api/core` — `invoke()` returns resolved promises
- `@tauri-apps/api/event` — `listen()` returns cleanup function
- `@tauri-apps/plugin-dialog` — `open()` / `save()` return paths
- `@tauri-apps/api/window` — `onDragDropEvent` returns unlisten

## Test Organization
```
src/__tests__/
  setup.ts              — global mocks and polyfills
  utils.test.ts         — cn() utility
  App.test.tsx          — top-level integration
  FileDropZone.test.tsx — file selection
  OutputDirectory.test.tsx — directory picker
  QueueView.test.tsx    — file list rendering
  LogPanel.test.tsx     — activity log
  AdvancedOptions.test.tsx — settings profiles
  Header.test.tsx       — greeting and theme
  types.test.ts         — TypeScript type validation
```

## Coverage Targets
- Components: all states (empty, populated, error, loading)
- Interactions: click, type, drag (where possible)
- Events: correct handlers called with expected args
- Types: compile-time validation for all interfaces

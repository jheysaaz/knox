# LogPanel — Activity Log Component

## User Journey
1. User sees a card titled "Activity" with timestamped log entries
2. Logs are color-coded by severity (info=blue, warn=yellow, error=red)
3. Logs auto-scroll to show latest entries
4. User can save logs to a file via download button

## Props
- `logs: LogEntry[]` — array of log entries

## Log Format
- Timestamp: `HH:MM:SS`
- Level: `[INFO]`, `[WARN]`, `[ERROR]` (color-coded)
- Message: free text

## Behavior
- Auto-scrolls to bottom when new logs arrive
- Save button opens save dialog with `.log` filter
- Saved content format: `[HH:MM:SS] [LEVEL] message\n`

## Acceptance Criteria
- Empty state shows "No activity yet"
- Each log shows timestamp, severity label, and message
- Severity levels have distinct colors
- Auto-scrolls to latest log
- Save calls write_log_file Tauri command with formatted content
- Save silently fails if dialog returns null

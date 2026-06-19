import { invoke } from '@tauri-apps/api/core';
import { save } from '@tauri-apps/plugin-dialog';
import { Download } from 'lucide-react';
import { useCallback, useEffect, useRef } from 'react';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { cn } from '@/lib/utils';
import { type LogEntry } from '@/types';

/** Displays session activity logs with save-to-file support. */
interface LogPanelProps {
  logs: LogEntry[];
}

/** Renders session log entries with auto-scroll and save-to-file capability. */
export function LogPanel({ logs }: LogPanelProps) {
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, []);

  const levelColors: Record<LogEntry['level'], string> = {
    info: 'text-blue-500',
    warn: 'text-yellow-600',
    error: 'text-destructive',
  };

  const levelLabel: Record<LogEntry['level'], string> = {
    info: 'INFO',
    warn: 'WARN',
    error: 'ERROR',
  };

  const formatTime = (date: Date) => {
    const h = date.getHours().toString().padStart(2, '0');
    const m = date.getMinutes().toString().padStart(2, '0');
    const s = date.getSeconds().toString().padStart(2, '0');
    return `${h}:${m}:${s}`;
  };

  const handleSave = useCallback(async () => {
    const path = await save({
      filters: [{ name: 'Log', extensions: ['log'] }],
      defaultPath: 'session.log',
    });
    if (!path) return;

    const lines = logs.map(
      (log) =>
        `[${formatTime(log.timestamp)}] [${levelLabel[log.level].padEnd(5)}] ${log.message}`,
    );
    const content = `${lines.join('\n')}\n`;

    try {
      await invoke('write_log_file', { path, content });
    } catch {
      // silently fail — could add a toast system later
    }
  }, [logs, levelLabel, formatTime]);

  return (
    <Card size="sm" className="overflow-hidden flex flex-col h-full">
      <CardHeader className="flex flex-row items-center justify-between">
        <CardTitle className="text-sm font-medium">Activity</CardTitle>
        <Button
          variant="ghost"
          size="icon"
          onClick={handleSave}
          title="Save logs"
        >
          <Download className="h-4 w-4" />
        </Button>
      </CardHeader>
      <CardContent ref={scrollRef} className="flex-1 overflow-y-auto min-h-0">
        <div className="space-y-1 font-mono text-xs leading-relaxed">
          {logs.length === 0 ? (
            <span className="text-muted-foreground italic">
              No activity yet
            </span>
          ) : (
            logs.map((log) => (
              <div
                key={log.id}
                className="rounded-md bg-muted/50 px-2 py-1.5 overflow-hidden"
              >
                <span className="float-left mr-2 text-muted-foreground">
                  {formatTime(log.timestamp)}
                </span>
                <span
                  className={cn(
                    'float-left font-semibold',
                    levelColors[log.level],
                  )}
                >
                  [{levelLabel[log.level]}]
                </span>
                <span className="text-foreground/80 break-words">
                  {log.message}
                </span>
              </div>
            ))
          )}
        </div>
      </CardContent>
    </Card>
  );
}

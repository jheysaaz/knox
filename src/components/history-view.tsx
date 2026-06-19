import { AlertCircle, Ban, CheckCircle2 } from 'lucide-react';
import { Card, CardContent } from '@/components/ui/card';
import { cn } from '@/lib/utils';
import type { HistoryEntry } from '@/types';

interface HistoryViewProps {
  entries: HistoryEntry[];
}

function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  const s = ms / 1000;
  if (s < 60) return `${s.toFixed(1)}s`;
  const m = Math.floor(s / 60);
  const secs = s % 60;
  return `${m}m ${secs.toFixed(0)}s`;
}

function formatTimestamp(unixMs: number): string {
  const d = new Date(unixMs);
  return d.toLocaleTimeString(undefined, {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  });
}

function fileName(path: string): string {
  const parts = path.replace(/\\/g, '/').split('/');
  return parts[parts.length - 1] || path;
}

export function HistoryView({ entries }: HistoryViewProps) {
  const statusIcon = (status: HistoryEntry['status']) => {
    switch (status) {
      case 'completed':
        return <CheckCircle2 className="h-4 w-4 text-green-500 shrink-0" />;
      case 'failed':
        return <AlertCircle className="h-4 w-4 text-destructive shrink-0" />;
      case 'cancelled':
        return <Ban className="h-4 w-4 text-amber-500 shrink-0" />;
    }
  };

  const statusLabel = (status: HistoryEntry['status']) => {
    switch (status) {
      case 'completed':
        return 'Completed';
      case 'failed':
        return 'Failed';
      case 'cancelled':
        return 'Cancelled';
    }
  };

  return (
    <Card size="sm" className="flex flex-col overflow-hidden h-full">
      <CardContent className="flex-1 overflow-y-auto px-3 pb-3">
        {entries.length === 0 ? (
          <p className="text-sm text-muted-foreground text-center py-8">
            No history yet
          </p>
        ) : (
          <div className="space-y-2">
            {entries.map((entry) => (
              <div
                key={entry.id}
                className={cn(
                  'flex items-start gap-3 rounded-md border px-3 py-2',
                  entry.status === 'failed'
                    ? 'border-destructive/50 bg-destructive/5'
                    : entry.status === 'cancelled'
                      ? 'border-amber-500/50 bg-amber-500/5'
                      : 'border-border bg-card',
                )}
              >
                {statusIcon(entry.status)}
                <div className="min-w-0 flex-1">
                  <p className="truncate text-sm text-foreground">
                    {fileName(entry.inputPath)}
                  </p>
                  <div className="flex items-center gap-2 mt-0.5">
                    <span
                      className={cn(
                        'text-xs',
                        entry.status === 'completed' && 'text-green-500',
                        entry.status === 'failed' && 'text-destructive',
                        entry.status === 'cancelled' && 'text-amber-500',
                      )}
                    >
                      {statusLabel(entry.status)}
                    </span>
                    <span className="text-xs text-muted-foreground">·</span>
                    <span className="text-xs text-muted-foreground">
                      {formatDuration(entry.durationMs)}
                    </span>
                    <span className="text-xs text-muted-foreground">·</span>
                    <span className="text-xs text-muted-foreground">
                      {formatTimestamp(entry.finishedAt)}
                    </span>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}

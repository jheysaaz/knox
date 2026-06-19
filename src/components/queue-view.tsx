import {
  AlertCircle,
  Ban,
  CheckCircle2,
  FileText,
  Loader2,
  PauseCircle,
  RotateCw,
  Square,
  X,
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Progress } from '@/components/ui/progress';
import { cn } from '@/lib/utils';
import { type FileItem } from '@/types';

/** Displays the file processing queue with status, progress, and controls. */
interface QueueViewProps {
  files: FileItem[];
  onFileRemove: (id: string) => void;
  onClear: () => void;
  onReprocess?: (id: string) => void;
  onStop?: () => void;
  isRunning?: boolean;
}

/** Displays the queue of files to process with per-file progress and action buttons. */
export function QueueView({
  files,
  onFileRemove,
  onClear,
  onReprocess,
  onStop,
  isRunning = true,
}: QueueViewProps) {
  const formatFileSize = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`;
    const kb = bytes / 1024;
    if (kb < 1000) return `${+kb.toFixed(1)} KB`;
    return `${+(kb / 1024).toFixed(1)} MB`;
  };

  const statusIcon = (status: FileItem['status'], running: boolean) => {
    switch (status) {
      case 'processing':
        return running ? (
          <Loader2 className="h-4 w-4 animate-spin text-blue-500" />
        ) : (
          <PauseCircle className="h-4 w-4 text-amber-500" />
        );
      case 'complete':
        return <CheckCircle2 className="h-4 w-4 text-green-500" />;
      case 'paused':
        return <Ban className="h-4 w-4 text-amber-500" />;
      case 'error':
        return <AlertCircle className="h-4 w-4 text-destructive" />;
      default:
        return <FileText className="h-4 w-4 text-muted-foreground" />;
    }
  };

  const statusLabel = (status: FileItem['status'], running: boolean) => {
    switch (status) {
      case 'processing':
        return running ? 'Processing...' : 'Pausing...';
      case 'complete':
        return 'Complete';
      case 'paused':
        return 'Paused';
      case 'error':
        return 'Error';
      default:
        return 'Pending';
    }
  };

  return (
    <Card size="sm" className="flex flex-col overflow-hidden h-full">
      <CardHeader className="flex flex-row items-center justify-between px-3 py-1.5">
        <CardTitle className="text-sm font-medium">Queue</CardTitle>
        <div className="flex items-center gap-1">
          {isRunning && (
            <Button
              variant="ghost"
              size="sm"
              onClick={onStop}
              className="h-auto py-1 px-2 text-xs text-destructive hover:text-destructive"
            >
              <Square className="h-3 w-3 mr-1" />
              Pause
            </Button>
          )}
          {files.length > 0 && (
            <Button
              variant="ghost"
              size="sm"
              onClick={onClear}
              disabled={files.some((f) => f.status === 'processing')}
              className="h-auto py-1 px-2 text-xs text-muted-foreground hover:text-foreground disabled:opacity-50"
            >
              Clear
            </Button>
          )}
        </div>
      </CardHeader>
      <CardContent className="flex-1 overflow-y-auto px-3 pb-3">
        {files.length === 0 ? (
          <p className="text-sm text-muted-foreground text-center py-8">
            No files added yet
          </p>
        ) : (
          <div className="space-y-2">
            {files.map((file) => (
              <div
                key={file.id}
                className={cn(
                  'flex items-center gap-3 rounded-md border px-3 py-2',
                  file.status === 'error'
                    ? 'border-destructive/50 bg-destructive/5'
                    : file.status === 'paused'
                      ? 'border-amber-500/50 bg-amber-500/5'
                      : !isRunning && file.status === 'processing'
                        ? 'border-amber-500/50 bg-amber-500/5'
                        : 'border-border bg-card',
                )}
              >
                {statusIcon(file.status, isRunning)}
                <div className="min-w-0 flex-1">
                  <p className="truncate text-sm text-foreground">
                    {file.name}
                  </p>
                  <div className="flex items-center gap-2">
                    <p className="text-xs text-muted-foreground">
                      {formatFileSize(file.size)}
                    </p>
                    <span className="text-xs text-muted-foreground">·</span>
                    <span
                      className={cn(
                        'text-xs',
                        file.status === 'paused' && 'text-amber-500',
                        file.status === 'processing' &&
                          !isRunning &&
                          'text-amber-500',
                        file.status === 'processing' &&
                          isRunning &&
                          'text-blue-500',
                        file.status === 'complete' && 'text-green-500',
                        file.status === 'error' && 'text-destructive',
                        file.status === 'pending' && 'text-muted-foreground',
                      )}
                    >
                      {statusLabel(file.status, isRunning)}
                    </span>
                  </div>
                  {file.status === 'processing' && (
                    <Progress className="mt-1 h-1" value={file.progress ?? 0} />
                  )}
                </div>
                {(file.status === 'complete' ||
                  file.status === 'error' ||
                  file.status === 'paused') && (
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => onReprocess?.(file.id)}
                    className="h-4 w-4 shrink-0 p-0 text-muted-foreground hover:text-foreground"
                    title="Reprocess"
                  >
                    <RotateCw className="h-3 w-3" />
                  </Button>
                )}
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => onFileRemove(file.id)}
                  className="h-4 w-4 shrink-0 p-0 text-muted-foreground hover:text-foreground"
                >
                  <X className="h-3 w-3" />
                </Button>
              </div>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}

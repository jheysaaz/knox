import { History, List, Square, Terminal, Trash2 } from 'lucide-react';
import { HistoryView } from '@/components/history-view';
import { LogPanel } from '@/components/log-panel';
import { QueueView } from '@/components/queue-view';
import { Button } from '@/components/ui/button';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import type { FileItem, HistoryEntry, LogEntry } from '@/types';

interface RightPanelProps {
  files: FileItem[];
  onFileRemove: (id: string) => void;
  onClear: () => void;
  onReprocess: (id: string) => void;
  isRunning: boolean;
  onStop: () => void;
  logs: LogEntry[];
  history: HistoryEntry[];
  onClearHistory: () => void;
  activeTab: string;
  onTabChange: (tab: string) => void;
}

function Badge({ count }: { count: number }) {
  if (count === 0) return null;
  return (
    <span className="inline-flex items-center justify-center h-4 min-w-4 px-[3px] rounded-full bg-muted-foreground/15 text-[10px] font-bold text-muted-foreground leading-none">
      {count > 99 ? 99 : count}
    </span>
  );
}

function RightPanel({
  files,
  onFileRemove,
  onClear,
  onReprocess,
  isRunning,
  onStop,
  logs,
  history,
  onClearHistory,
  activeTab,
  onTabChange,
}: RightPanelProps) {
  return (
    <Tabs
      value={activeTab}
      onValueChange={onTabChange}
      className="flex flex-col min-h-0 flex-1"
    >
      <div className="flex items-center gap-2 mb-2">
        <TabsList className="flex-1">
          <TabsTrigger value="queue" className="gap-1.5">
            <List className="h-4 w-4" />
            Queue
            <Badge count={files.length} />
          </TabsTrigger>
          <TabsTrigger value="history" className="gap-1.5">
            <History className="h-4 w-4" />
            History
            <Badge count={history.length} />
          </TabsTrigger>
          <TabsTrigger value="activity" className="gap-1.5">
            <Terminal className="h-4 w-4" />
            Activity
          </TabsTrigger>
        </TabsList>

        {activeTab === 'queue' && files.length > 0 && (
          <div className="flex items-center gap-1 shrink-0">
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
            <Button
              variant="ghost"
              size="sm"
              onClick={onClear}
              disabled={files.some((f) => f.status === 'processing')}
              className="h-auto py-1 px-2 text-xs text-muted-foreground hover:text-foreground disabled:opacity-50"
            >
              Clear
            </Button>
          </div>
        )}
        {activeTab === 'history' && history.length > 0 && (
          <div className="flex items-center gap-1 shrink-0">
            <Button
              variant="ghost"
              size="sm"
              onClick={onClearHistory}
              className="h-auto py-1 px-2 text-xs text-muted-foreground hover:text-foreground"
            >
              <Trash2 className="h-3 w-3 mr-1" />
              Clear
            </Button>
          </div>
        )}
      </div>

      <TabsContent
        value="queue"
        className="flex-1 min-h-0 mt-0 data-[state=active]:flex flex-col"
      >
        <QueueView
          files={files}
          onFileRemove={onFileRemove}
          onReprocess={onReprocess}
          isRunning={isRunning}
        />
      </TabsContent>

      <TabsContent
        value="history"
        className="flex-1 min-h-0 mt-0 data-[state=active]:flex flex-col"
      >
        <HistoryView entries={history} />
      </TabsContent>

      <TabsContent
        value="activity"
        className="flex-1 min-h-0 mt-0 data-[state=active]:flex flex-col"
      >
        <LogPanel logs={logs} />
      </TabsContent>
    </Tabs>
  );
}

export default RightPanel;

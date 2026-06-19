import { QueueView } from "@/components/queue-view";
import { LogPanel } from "@/components/log-panel";
import { HistoryView } from "@/components/history-view";
import type { FileItem, LogEntry, HistoryEntry } from "@/types";

interface RightPanelProps {
  files: FileItem[];
  onFileRemove: (id: string) => void;
  onClear: () => void;
  onReprocess: (id: string) => void;
  isRunning: boolean;
  onStop: () => void;
  showActivity: boolean;
  showHistory: boolean;
  logs: LogEntry[];
  history: HistoryEntry[];
  onClearHistory: () => void;
}

function RightPanel({
  files,
  onFileRemove,
  onClear,
  onReprocess,
  isRunning,
  onStop,
  showActivity,
  showHistory,
  logs,
  history,
  onClearHistory,
}: RightPanelProps) {
  if (showHistory) {
    return (
      <div className="flex-1 min-h-0">
        <HistoryView entries={history} onClear={onClearHistory} />
      </div>
    );
  }

  return (
    <>
      {showActivity ? (
        <>
          <div className="flex-[3] min-h-0">
            <QueueView
              files={files}
              onFileRemove={onFileRemove}
              onClear={onClear}
              onReprocess={onReprocess}
              isRunning={isRunning}
              onStop={onStop}
            />
          </div>
          <div className="flex-[1] min-h-0">
            <LogPanel logs={logs} />
          </div>
        </>
      ) : (
        <div className="flex-1 min-h-0">
          <QueueView
            files={files}
            onFileRemove={onFileRemove}
            onClear={onClear}
            onReprocess={onReprocess}
            isRunning={isRunning}
            onStop={onStop}
          />
        </div>
      )}
    </>
  );
}

export default RightPanel;

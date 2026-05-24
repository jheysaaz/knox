import { QueueView } from "@/components/queue-view";
import { LogPanel } from "@/components/log-panel";
import type { FileItem, LogEntry } from "@/types";

interface RightPanelProps {
  files: FileItem[];
  onFileRemove: (id: string) => void;
  onClear: () => void;
  onReprocess: (id: string) => void;
  isRunning: boolean;
  onStop: () => void;
  showActivity: boolean;
  logs: LogEntry[];
}

function RightPanel({
  files,
  onFileRemove,
  onClear,
  onReprocess,
  isRunning,
  onStop,
  showActivity,
  logs,
}: RightPanelProps) {
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

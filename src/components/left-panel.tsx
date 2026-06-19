import { Play } from 'lucide-react';
import { useEffect } from 'react';
import {
  AdvancedOptions,
  type ProfileValues,
} from '@/components/advanced-options';
import { FileDropZone } from '@/components/file-dropzone';
import { OutputDirectory } from '@/components/output-directory';
import { Button } from '@/components/ui/button';
import { Spinner } from '@/components/ui/spinner';
import type { FileItem } from '@/types';

interface LeftPanelProps {
  onFilesAdded: (files: FileItem[]) => void;
  outputDir: string;
  onOutputDirChange: (dir: string) => void;
  settings: ProfileValues;
  onSettingsChange: (next: ProfileValues) => void;
  isRunning: boolean;
  starting: boolean;
  onStart: (settings: ProfileValues) => void;
}

function LeftPanel({
  onFilesAdded,
  outputDir,
  onOutputDirChange,
  settings,
  onSettingsChange,
  isRunning,
  starting,
  onStart,
}: LeftPanelProps) {
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.metaKey && e.key === 'Enter') {
        e.preventDefault();
        onStart(settings);
      }
    };
    document.addEventListener('keydown', handler);
    return () => document.removeEventListener('keydown', handler);
  }, [onStart, settings]);
  return (
    <div className="space-y-4">
      <FileDropZone onFilesAdded={onFilesAdded} />
      <OutputDirectory value={outputDir} onChange={onOutputDirChange} />
      <AdvancedOptions value={settings} onChange={onSettingsChange} />
      <Button
        className="w-full"
        size="lg"
        variant="default"
        onClick={() => onStart(settings)}
        disabled={starting}
      >
        {starting ? (
          <span className="flex items-center gap-2">
            <Spinner /> Starting…
          </span>
        ) : isRunning ? (
          'Add to Queue'
        ) : (
          <span className="flex items-center gap-2">
            <Play className="h-4 w-4" />
            Start Queue
          </span>
        )}
      </Button>
    </div>
  );
}

export default LeftPanel;

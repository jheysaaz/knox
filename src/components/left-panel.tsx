import { FileDropZone } from "@/components/file-dropzone";
import { OutputDirectory } from "@/components/output-directory";
import { AdvancedOptions, type ProfileValues } from "@/components/advanced-options";
import { Button } from "@/components/ui/button";
import type { FileItem } from "@/types";

interface LeftPanelProps {
  onFilesAdded: (files: FileItem[]) => void;
  outputDir: string;
  onOutputDirChange: (dir: string) => void;
  settings: ProfileValues;
  onSettingsChange: (next: ProfileValues) => void;
  isRunning: boolean;
  onStart: (settings: ProfileValues) => void;
}

function LeftPanel({
  onFilesAdded,
  outputDir,
  onOutputDirChange,
  settings,
  onSettingsChange,
  isRunning,
  onStart,
}: LeftPanelProps) {
  return (
    <div className="space-y-4">
      <FileDropZone onFilesAdded={onFilesAdded} />
      <OutputDirectory value={outputDir} onChange={onOutputDirChange} />
      <AdvancedOptions value={settings} onChange={onSettingsChange} />
      <Button className="w-full" size="lg" onClick={() => onStart(settings)}>
        {isRunning ? "Add to Queue" : "Start OCR Processing"}
      </Button>
    </div>
  );
}

export default LeftPanel;

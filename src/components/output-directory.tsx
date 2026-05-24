import { useCallback } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { FolderOpen } from "lucide-react";

/** Controls for selecting and displaying the output directory. */
interface OutputDirectoryProps {
  value: string;
  onChange: (value: string) => void;
}

/** Directory picker with text input and browse button. */
export function OutputDirectory({ value, onChange }: OutputDirectoryProps) {
  const handleBrowse = useCallback(async () => {
    try {
      const selected = await open({
        directory: true,
        title: "Select Output Directory",
      });
      if (selected) {
        onChange(selected);
      }
    } catch (err) {
      console.error("Directory dialog error:", err);
    }
  }, [onChange]);

  return (
    <div className="space-y-2">
      <label className="text-sm font-medium text-foreground">
        Output Directory
      </label>
      <div className="flex gap-2">
        <Input
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder="Select output directory..."
          className="flex-1"
        />
        <Button
          variant="outline"
          size="icon"
          onClick={handleBrowse}
          title="Browse"
        >
          <FolderOpen className="h-4 w-4" />
        </Button>
      </div>
    </div>
  );
}

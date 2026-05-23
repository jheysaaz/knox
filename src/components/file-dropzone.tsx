import { useCallback, useEffect, useState } from "react";
import { Upload } from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { cn } from "@/lib/utils";
import { type FileItem } from "@/types";

interface FileDropZoneProps {
  onFilesAdded: (files: FileItem[]) => void;
}

async function pathsToFileItems(paths: string[]): Promise<FileItem[]> {
  const items: FileItem[] = [];
  for (const path of paths) {
    if (!path.toLowerCase().endsWith(".pdf")) continue;
    let size = 0;
    try {
      const metadata = await invoke<{ size: number }>("get_file_metadata", {
        path,
      });
      size = metadata.size;
    } catch {}
    items.push({
      id: crypto.randomUUID(),
      path,
      name: path.split("/").pop() || path,
      size,
      status: "pending" as const,
    });
  }
  return items;
}

export function FileDropZone({ onFilesAdded }: FileDropZoneProps) {
  const [isDragging, setIsDragging] = useState(false);

  useEffect(() => {
    const unlistenPromise = getCurrentWindow().onDragDropEvent((event) => {
      const { type } = event.payload;

      if (type === "enter" || type === "over") {
        setIsDragging(true);
      } else if (type === "drop") {
        setIsDragging(false);
        pathsToFileItems(event.payload.paths).then((items) => {
          if (items.length > 0) onFilesAdded(items);
        });
      } else if (type === "leave") {
        setIsDragging(false);
      }
    });

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, [onFilesAdded]);

  const handleBrowse = useCallback(async () => {
    try {
      const selected = await open({
        multiple: true,
        filters: [{ name: "PDF", extensions: ["pdf"] }],
      });
      if (selected) {
        const paths = Array.isArray(selected) ? selected : [selected];
        const items = await pathsToFileItems(paths);
        if (items.length > 0) onFilesAdded(items);
      }
    } catch (err) {
      console.error("File dialog error:", err);
    }
  }, [onFilesAdded]);

  return (
    <div>
      <div
        onClick={handleBrowse}
        role="button"
        tabIndex={0}
        onKeyDown={(e) => e.key === "Enter" && handleBrowse()}
        className={cn(
          "flex flex-col items-center justify-center rounded-lg border border-dashed p-16 transition-all duration-200 cursor-pointer",
          isDragging
            ? "border-foreground bg-muted"
            : "border-border hover:border-foreground/40 hover:bg-muted/50",
        )}
      >
        <div
          className={cn(
            "flex flex-col items-center gap-3 transition-transform duration-200",
            isDragging && "scale-105",
          )}
        >
          <Upload
            className={cn(
              "h-8 w-8 transition-colors",
              isDragging ? "text-foreground" : "text-muted-foreground",
            )}
          />
          <div className="text-center">
            <p className="text-sm font-medium text-foreground">
              {isDragging ? "Drop files here" : "Drop PDF files here"}
            </p>
            <p className="mt-1 text-xs text-muted-foreground">
              or click to browse
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}

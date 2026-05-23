export interface FileItem {
  id: string;
  path: string;
  name: string;
  size: number;
  status: "pending" | "processing" | "complete" | "error" | "paused";
  progress?: number;
  queued?: boolean;
}

export type BinarizationMode = "otsu" | "bradley-roth" | "fixed";
export type DeskewMode = "radon" | "hough" | "disabled";
export type ExistingTextMode = "skip" | "rasterize";
export type PageSegMode = "auto" | "block" | "column" | "sparse";
export type CompressionMode = "ccitt" | "flate";

export interface OcrSettings {
  cpuCores: number;
  memoryPages: number;
  binarization: BinarizationMode;
  fixedThreshold: number;
  deskew: DeskewMode;
  denoiseLevel: number;
  existingText: ExistingTextMode;
  psm: PageSegMode;
  compression: CompressionMode;
  resolution: string;
  archiveEnforcement: boolean;
  languages: string;
}

export interface LogEntry {
  id: string;
  timestamp: Date;
  level: "info" | "warn" | "error";
  message: string;
}

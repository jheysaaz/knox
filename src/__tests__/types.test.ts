import { describe, it, expect } from "vitest";
import type { FileItem, LogEntry, OcrSettings } from "@/types";

describe("types", () => {
  it("FileItem can be constructed", () => {
    const file: FileItem = {
      id: "1",
      path: "/path/to/file.pdf",
      name: "file.pdf",
      size: 1024,
      status: "pending",
    };
    expect(file.status).toBe("pending");
  });

  it("FileItem supports all statuses", () => {
    const statuses: FileItem["status"][] = [
      "pending",
      "processing",
      "complete",
      "error",
      "paused",
    ];
    for (const s of statuses) {
      const file: FileItem = {
        id: "1",
        path: "/p.pdf",
        name: "p.pdf",
        size: 0,
        status: s,
      };
      expect(file.status).toBe(s);
    }
  });

  it("LogEntry can be constructed", () => {
    const log: LogEntry = {
      id: "1",
      timestamp: new Date(),
      level: "info",
      message: "test",
    };
    expect(log.level).toBe("info");
  });

  it("OcrSettings can be constructed", () => {
    const settings: OcrSettings = {
      cpuCores: 6,
      memoryPages: 30,
      binarization: "otsu",
      fixedThreshold: 128,
      deskew: "radon",
      denoiseLevel: 2,
      existingText: "skip",
      psm: "auto",
      compression: "ccitt",
      resolution: "300",
      archiveEnforcement: false,
      languages: "eng",
    };
    expect(settings.cpuCores).toBe(6);
  });
});

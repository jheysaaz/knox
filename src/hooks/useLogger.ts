import { useState } from "react";
import type { LogEntry } from "@/types";

const MAX_LOG_ENTRIES = 500;

export function useLogger() {
  const [logs, setLogs] = useState<LogEntry[]>([]);

  const addLog = (level: LogEntry["level"], message: string) => {
    setLogs((prev) => {
      const next = [
        ...prev,
        { id: crypto.randomUUID(), timestamp: new Date(), level, message },
      ];
      return next.length > MAX_LOG_ENTRIES
        ? next.slice(next.length - MAX_LOG_ENTRIES)
        : next;
    });
  };

  return { logs, addLog };
}

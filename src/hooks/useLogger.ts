import { useState } from "react";
import type { LogEntry } from "@/types";

export function useLogger() {
  const [logs, setLogs] = useState<LogEntry[]>([]);

  const addLog = (level: LogEntry["level"], message: string) => {
    setLogs((prev) => [
      ...prev,
      { id: crypto.randomUUID(), timestamp: new Date(), level, message },
    ]);
  };

  return { logs, addLog };
}

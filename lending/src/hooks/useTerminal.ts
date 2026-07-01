import { useState, useEffect } from "react";
import type { TerminalLogItem } from "../utils/types";
import { TERMINAL_MOCK_MESSAGES } from "../utils/constants";

export function useTerminal() {
  const [logs, setLogs] = useState<TerminalLogItem[]>([]);

  useEffect(() => {
    let index = 0;
    
    const addNextLog = () => {
      const msg = TERMINAL_MOCK_MESSAGES[index % TERMINAL_MOCK_MESSAGES.length];
      const now = new Date();
      const timeStr = now.toTimeString().split(" ")[0];
      
      let type: "info" | "success" | "warn" | "error" | "system" = "info";
      if (msg.includes("ERROR") || msg.includes("FAIL")) {
        type = "error";
      } else if (msg.includes("REPUTATION") || msg.includes("COMPLETED")) {
        type = "success";
      } else if (msg.includes("ESCROW") || msg.includes("LOCKED")) {
        type = "warn";
      } else if (msg.includes("INITIALIZING") || msg.includes("CONNECTING")) {
        type = "system";
      }

      const newItem: TerminalLogItem = {
        id: Math.random().toString(36).substring(2, 9),
        timestamp: timeStr,
        type,
        message: msg,
      };

      setLogs((prev) => {
        const next = [...prev, newItem];
        return next.slice(-10);
      });

      index++;
      const nextDelay = 1500 + Math.random() * 2000;
      timer = setTimeout(addNextLog, nextDelay);
    };

    let timer = setTimeout(addNextLog, 500);
    return () => clearTimeout(timer);
  }, []);

  return logs;
}

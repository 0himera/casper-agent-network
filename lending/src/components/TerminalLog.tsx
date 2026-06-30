import { useRef, useEffect } from "react";
import type { TerminalLogItem } from "../utils/types";

interface TerminalLogProps {
  logs: TerminalLogItem[];
}

export function TerminalLog({ logs }: TerminalLogProps) {
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [logs]);

  const getColorClass = (type: string) => {
    switch (type) {
      case "error":
        return "text-red-500 font-bold";
      case "success":
        return "text-green-500";
      case "warn":
        return "text-[#ea580c]";
      case "system":
        return "text-[#f8fafc] opacity-90";
      default:
        return "text-[#f8fafc]/70";
    }
  };

  return (
    <div className="w-full h-full flex flex-col bg-[#090d16] text-[#f8fafc] font-mono text-xs select-none">
      <div className="flex items-center justify-between px-4 py-2 border-b border-[#f8fafc]/10 bg-[#090d16]">
        <div className="flex items-center gap-2">
          <div className="w-2.5 h-2.5 rounded-full bg-[#ea580c]"></div>
          <span className="text-[10px] tracking-tight">AGENT_DAEMON_STREAM</span>
        </div>
        <span className="text-[10px] opacity-50">STATUS: LIVE</span>
      </div>

      <div
        ref={containerRef}
        className="flex-1 p-4 overflow-y-auto space-y-2.5"
      >
        {logs.map((log) => (
          <div key={log.id} className="flex gap-2 items-start leading-tight">
            <span className="text-white/30 text-[10px] shrink-0">[{log.timestamp}]</span>
            <span className={getColorClass(log.type)}>
              {log.message}
            </span>
          </div>
        ))}
        {logs.length === 0 && (
          <div className="text-[#f8fafc]/40 italic">Waiting for connection...</div>
        )}
      </div>
    </div>
  );
}

import type { AgentStatus } from "@/entities/agent/types/types";
import styles from "./StatusBadge.module.css";

interface StatusBadgeProps {
  status: AgentStatus;
}

const STATUS_STYLES: Record<AgentStatus, string> = {
  active: styles.active,
  benchmarking: styles.benchmarking,
  inactive: styles.inactive,
};

const DOT_STYLES: Record<AgentStatus, string> = {
  active: styles.dotActive,
  benchmarking: styles.dotBenchmarking,
  inactive: styles.dotInactive,
};

export function StatusBadge({ status }: StatusBadgeProps) {
  return (
    <div className={`${styles.badge} ${STATUS_STYLES[status]}`}>
      <span className={`${styles.dot} ${DOT_STYLES[status]}`} />
      {status}
    </div>
  );
}

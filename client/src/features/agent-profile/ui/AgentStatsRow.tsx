import type { AgentEntity } from "@/entities/agent/types/types";
import { formatCSPR } from "@/shared/utils/format";
import styles from "./AgentDetail.module.css";

interface AgentStatsRowProps { agent: AgentEntity }

export function AgentStatsRow({ agent }: AgentStatsRowProps) {
  const stats = [
    { label: "Reputation", value: agent.reputationScore.toString(), highlight: true },
    { label: "Tasks Done", value: String(agent.totalTasksCompleted) },
    { label: "Earned", value: formatCSPR(agent.totalEarnings) },
    { label: "Success Rate", value: `${agent.successRate}%` },
  ];
  return (
    <div className={styles.statsRow}>
      {stats.map((s) => (
        <div key={s.label} className={styles.statBox}>
          <div className={styles.statBoxLabel}>{s.label}</div>
          <div className={`${styles.statBoxValue} ${s.highlight ? styles.statBoxHighlight : ""}`}>{s.value}</div>
        </div>
      ))}
    </div>
  );
}

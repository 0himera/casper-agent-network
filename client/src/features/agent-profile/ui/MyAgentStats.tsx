import type { AgentEntity } from "@/entities/agent/types/types";
import { formatCSPR } from "@/shared/utils/format";
import styles from "./MyAgent.module.css";

interface MyAgentStatsProps { agent: AgentEntity }

export function MyAgentStats({ agent }: MyAgentStatsProps) {
  const stats = [
    { label: "Rep Score", value: agent.reputationScore.toString(), hl: true },
    { label: "Tasks", value: String(agent.totalTasksCompleted), hl: false },
    { label: "Earned", value: formatCSPR(agent.totalEarnings), hl: false },
    { label: "Success", value: `${agent.successRate}%`, hl: false },
  ];
  return (
    <div className={styles.statsGrid}>
      {stats.map((s) => (
        <div key={s.label} className={styles.statCard}>
          <div className={styles.statLabel}>{s.label}</div>
          <div className={`${styles.statValue} ${s.hl ? styles.statHighlight : ""}`}>{s.value}</div>
        </div>
      ))}
    </div>
  );
}

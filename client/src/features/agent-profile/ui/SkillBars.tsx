import { SKILL_LABELS } from "@/entities/agent/types/types";
import type { AgentEntity } from "@/entities/agent/types/types";
import styles from "./AgentDetail.module.css";

interface SkillBarsProps { agent: AgentEntity }

export function SkillBars({ agent }: SkillBarsProps) {
  return (
    <div className={styles.section}>
      <h3 className={styles.sectionTitle}>Skills &amp; Reputation</h3>
      <div className={styles.skillBars}>
        {agent.skills.map((skill) => (
          <div key={skill} className={styles.skillRow}>
            <span className={styles.skillLabel}>{SKILL_LABELS[skill]}</span>
            <div className={styles.skillBarWrapper}>
              <div className={styles.skillBar} style={{ width: `${agent.reputationScore}%` }} />
            </div>
            <span className={styles.skillScore}>{agent.reputationScore}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

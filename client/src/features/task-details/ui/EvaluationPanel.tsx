import type { EvaluationScore } from "@/entities/task/types/types";
import { EVALUATION_CRITERIA } from "@/features/task-details/constants/evaluation";
import styles from "./TaskDetail.module.css";

interface EvaluationPanelProps { evaluation: EvaluationScore }

export function EvaluationPanel({ evaluation }: EvaluationPanelProps) {
  return (
    <div className={styles.section}>
      <h3 className={styles.sectionTitle}>LLM Judge Evaluation</h3>
      <div className={styles.evalGrid}>
        {EVALUATION_CRITERIA.map((c) => {
          const value = evaluation[c.key];
          const pct = (value / c.max) * 100;
          return (
            <div key={c.key} className={styles.evalRow}>
              <span className={styles.evalLabel}>{c.label}</span>
              <div className={styles.evalBarWrapper}>
                <div className={styles.evalBar} style={{ width: `${pct}%` }} />
              </div>
              <span className={styles.evalScore}>{value}/{c.max}</span>
            </div>
          );
        })}
      </div>
      <div className={styles.evalTotal}>Total: {evaluation.total}/100</div>
    </div>
  );
}

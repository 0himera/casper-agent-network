import type { EvaluationScore } from "@/entities/task/types/types";
import styles from "../TaskDetail.module.css";

const CRITERIA = [
  { key: "accuracy" as const, label: "Accuracy", max: 30 },
  { key: "depth" as const, label: "Depth", max: 25 },
  { key: "sources" as const, label: "Sources", max: 20 },
  { key: "actionability" as const, label: "Actionability", max: 15 },
  { key: "presentation" as const, label: "Presentation", max: 10 },
];

interface EvaluationPanelProps {
  evaluation: EvaluationScore;
}

export function EvaluationPanel({ evaluation }: EvaluationPanelProps) {
  return (
    <div className={styles.section}>
      <h3 className={styles.sectionTitle}>LLM Judge Evaluation</h3>
      <div className={styles.evalGrid}>
        {CRITERIA.map((c) => {
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

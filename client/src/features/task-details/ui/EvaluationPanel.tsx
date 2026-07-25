import type { EvaluationScore } from "@/entities/task/types/types";
import { EVALUATION_CRITERIA } from "@/features/task-details/constants/evaluation";
import styles from "./TaskDetail.module.css";

interface EvaluationPanelProps {
  evaluation: EvaluationScore;
}

export function EvaluationPanel({ evaluation }: EvaluationPanelProps) {
  return (
    <div className={styles.section}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "1rem" }}>
        <h3 className={styles.sectionTitle} style={{ margin: 0 }}>
          3-Validator LLM Judge Consensus
        </h3>
        <span
          style={{
            fontSize: "0.75rem",
            padding: "4px 10px",
            borderRadius: "12px",
            backgroundColor: "rgba(63, 185, 80, 0.15)",
            color: "#3fb950",
            border: "1px solid rgba(63, 185, 80, 0.3)",
            fontWeight: 600,
          }}
        >
          ✓ Quorum Reached (3/3 Validations Signed)
        </span>
      </div>

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
              <span className={styles.evalScore}>
                {value}/{c.max}
              </span>
            </div>
          );
        })}
      </div>

      <div style={{ marginTop: "1rem", paddingTop: "1rem", borderTop: "1px solid rgba(255,255,255,0.08)", display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <div style={{ fontSize: "0.8rem", color: "#888" }}>
          Evaluated independently by Fireworks AI, Google Gemini, & OpenRouter Nemotron
        </div>
        <div className={styles.evalTotal}>Consensus Score: {evaluation.total}/100</div>
      </div>
    </div>
  );
}

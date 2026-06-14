import { Check } from "lucide-react";
import styles from "../TaskDetail.module.css";

interface Step {
  label: string;
  time: string | null;
  detail?: string;
}

interface StatusTimelineProps {
  steps: Step[];
}

export function StatusTimeline({ steps }: StatusTimelineProps) {
  return (
    <div className={styles.section}>
      <h3 className={styles.sectionTitle}>Status Timeline</h3>
      <div className={styles.timeline}>
        {steps.map((step, i) => {
          const done = !!step.time;
          return (
            <div key={i} className={styles.timelineStep}>
              <div className={`${styles.timelineDot} ${done ? styles.dotCompleted : styles.dotPending}`}>
                {done ? <Check size={14} /> : i + 1}
              </div>
              <div className={styles.timelineContent}>
                <span className={styles.timelineLabel}>{step.label}</span>
                <span className={styles.timelineTime}>
                  {step.time ?? "Pending"}{step.detail ? ` → ${step.detail}` : ""}
                </span>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

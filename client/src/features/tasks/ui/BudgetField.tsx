import { AlertTriangle, Lightbulb } from "lucide-react";
import styles from "./CreateTask.module.css";

interface BudgetFieldProps {
  value: string;
  onChange: (v: string) => void;
  recommended: number;
}

export function BudgetField({ value, onChange, recommended }: BudgetFieldProps) {
  const isTooLow = Number(value) < 1;

  return (
    <div className={styles.field}>
      <label className={styles.label}>Budget (CSPR)</label>
      <input
        type="number"
        className={styles.input}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        min="1"
        step="0.1"
        placeholder="5.0"
      />
      <div className={styles.budgetMeta}>
        {isTooLow ? (
          <span className={styles.warning}>
            <AlertTriangle size={12} /> Minimum budget: 1.0 CSPR
          </span>
        ) : (
          <span className={styles.recommended}>
            <Lightbulb size={12} /> Recommended: {recommended} CSPR
          </span>
        )}
      </div>
    </div>
  );
}

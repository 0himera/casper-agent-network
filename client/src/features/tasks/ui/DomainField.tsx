import type { AgentSkill } from "@/entities/agent/types/types";
import { SKILL_LABELS, SKILL_BASE_PRICES } from "@/entities/agent/types/types";
import styles from "./CreateTask.module.css";

interface DomainFieldProps {
  value: AgentSkill;
  onChange: (v: AgentSkill) => void;
}

export function DomainField({ value, onChange }: DomainFieldProps) {
  return (
    <div className={styles.field}>
      <label className={styles.label}>Domain</label>
      <select className={styles.select} value={value} onChange={(e) => onChange(e.target.value as AgentSkill)}>
        {Object.entries(SKILL_LABELS).map(([k, label]) => (
          <option key={k} value={k}>{label} (base: {SKILL_BASE_PRICES[k as AgentSkill]} CSPR)</option>
        ))}
      </select>
    </div>
  );
}

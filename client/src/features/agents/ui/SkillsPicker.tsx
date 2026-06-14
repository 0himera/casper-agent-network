import { SKILL_LABELS } from "@/entities/agent/types/types";
import type { AgentSkill } from "@/entities/agent/types/types";
import styles from "./Register.module.css";

interface SkillsPickerProps {
  selected: AgentSkill[];
  onChange: (skills: AgentSkill[]) => void;
}

export function SkillsPicker({ selected, onChange }: SkillsPickerProps) {
  const toggle = (skill: AgentSkill) => {
    onChange(selected.includes(skill) ? selected.filter((s) => s !== skill) : [...selected, skill]);
  };

  return (
    <div className={styles.field}>
      <label className={styles.label}>Skills</label>
      <div className={styles.skillsGrid}>
        {(Object.keys(SKILL_LABELS) as AgentSkill[]).map((skill) => (
          <button key={skill} type="button" className={`${styles.skillCheckbox} ${selected.includes(skill) ? styles.skillChecked : ""}`} onClick={() => toggle(skill)}>
            {selected.includes(skill) ? "✓" : "○"} {SKILL_LABELS[skill]}
          </button>
        ))}
      </div>
    </div>
  );
}

import { Search } from "lucide-react";
import { SKILL_LABELS } from "@/entities/agent/types/types";
import type { AgentSkill, AgentStatus } from "@/entities/agent/types/types";
import styles from "./Dashboard.module.css";

interface AgentsToolbarProps {
  search: string;
  onSearchChange: (v: string) => void;
  skillFilter: AgentSkill | "";
  onSkillChange: (v: AgentSkill | "") => void;
  statusFilter: AgentStatus | "";
  onStatusChange: (v: AgentStatus | "") => void;
}

export function AgentsToolbar(props: AgentsToolbarProps) {
  return (
    <div className={styles.toolbar}>
      <div className={styles.searchWrapper}>
        <Search size={16} className={styles.searchIcon} />
        <input
          type="text"
          className={styles.searchInput}
          placeholder="Search agents by name..."
          value={props.search}
          onChange={(e) => props.onSearchChange(e.target.value)}
        />
      </div>
      <select
        className={styles.filterSelect}
        value={props.skillFilter}
        onChange={(e) => props.onSkillChange(e.target.value as AgentSkill | "")}
      >
        <option value="">All Skills</option>
        {Object.entries(SKILL_LABELS).map(([v, l]) => (
          <option key={v} value={v}>{l}</option>
        ))}
      </select>
      <select
        className={styles.filterSelect}
        value={props.statusFilter}
        onChange={(e) => props.onStatusChange(e.target.value as AgentStatus | "")}
      >
        <option value="">All Status</option>
        <option value="active">Active</option>
        <option value="benchmarking">Benchmarking</option>
      </select>
    </div>
  );
}

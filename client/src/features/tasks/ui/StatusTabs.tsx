import type { TaskStatus } from "@/entities/task/types/types";
import { STATUS_TABS } from "@/features/tasks/constants/status-tabs";
import styles from "./Tasks.module.css";

interface StatusTabsProps {
  active: TaskStatus | "all";
  onChange: (v: TaskStatus | "all") => void;
  counts: Record<string, number>;
}

export function StatusTabs({ active, onChange, counts }: StatusTabsProps) {
  return (
    <div className={styles.tabs}>
      {STATUS_TABS.map((tab) => (
        <button
          key={tab.value}
          className={`${styles.tab} ${active === tab.value ? styles.tabActive : ""}`}
          onClick={() => onChange(tab.value)}
        >
          {tab.label}
          <span className={styles.tabCount}>{counts[tab.value] ?? 0}</span>
        </button>
      ))}
    </div>
  );
}

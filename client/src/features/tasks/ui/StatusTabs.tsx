import type { TaskStatus } from "@/entities/task/types/types";
import { STATUS_TABS } from "@/features/tasks/constants/status-tabs";
import { motion } from "motion/react";
import styles from "./Tasks.module.css";

interface StatusTabsProps {
  active: TaskStatus | "all";
  onChange: (v: TaskStatus | "all") => void;
  counts: Record<string, number>;
}

export function StatusTabs({ active, onChange, counts }: StatusTabsProps) {
  return (
    <div className={styles.tabs}>
      {STATUS_TABS.map((tab) => {
        const isActive = active === tab.value;
        return (
          <button
            key={tab.value}
            className={`${styles.tab} ${isActive ? styles.tabActive : ""}`}
            onClick={() => onChange(tab.value)}
          >
            {isActive && (
              <motion.span
                layoutId="statusActivePill"
                className={styles.activePill}
                transition={{ type: "spring", stiffness: 380, damping: 30 }}
              />
            )}
            <span style={{ position: "relative", zIndex: 2 }}>{tab.label}</span>
            <span className={styles.tabCount} style={{ position: "relative", zIndex: 2 }}>
              {counts[tab.value] ?? 0}
            </span>
          </button>
        );
      })}
    </div>
  );
}

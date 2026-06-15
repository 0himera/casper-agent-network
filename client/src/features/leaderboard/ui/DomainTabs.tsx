import type { LeaderboardDomain } from "@/entities/reputation/types/types";
import { DOMAIN_TABS } from "@/features/leaderboard/constants/domains";
import styles from "./Leaderboard.module.css";

interface DomainTabsProps {
  active: LeaderboardDomain;
  onChange: (domain: LeaderboardDomain) => void;
}

export function DomainTabs({ active, onChange }: DomainTabsProps) {
  return (
    <div className={styles.tabs}>
      {DOMAIN_TABS.map((d) => (
        <button
          key={d.value}
          className={`${styles.tab} ${active === d.value ? styles.tabActive : ""}`}
          onClick={() => onChange(d.value)}
        >
          {d.label}
        </button>
      ))}
    </div>
  );
}

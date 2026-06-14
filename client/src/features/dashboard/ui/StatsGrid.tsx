import { STATS_CONFIG } from "@/features/dashboard/constants/stats";
import { StatCard } from "./StatCard";
import styles from "./Dashboard.module.css";

export function StatsGrid() {
  return (
    <div className={styles.statsGrid}>
      {STATS_CONFIG.map((stat) => (
        <StatCard key={stat.label} {...stat} />
      ))}
    </div>
  );
}

import type { LucideIcon } from "lucide-react";
import styles from "./Dashboard.module.css";

interface StatCardProps {
  label: string;
  value: string | number;
  icon: LucideIcon;
}

export function StatCard({ label, value, icon: Icon }: StatCardProps) {
  return (
    <div className={styles.statCard}>
      <div className={styles.statLabel}>{label}</div>
      <div className={styles.statValue}>{value}</div>
      <div className={styles.statIcon}>
        <Icon size={20} />
      </div>
    </div>
  );
}

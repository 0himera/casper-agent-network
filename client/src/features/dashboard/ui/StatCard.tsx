import type { LucideIcon } from "lucide-react";
import { motion } from "motion/react";
import styles from "./Dashboard.module.css";

interface StatCardProps {
  label: string;
  value: string | number;
  icon: LucideIcon;
}

const itemVariants = {
  hidden: { opacity: 0, scale: 0.95, y: 10 },
  show: {
    opacity: 1,
    scale: 1,
    y: 0,
    transition: {
      type: "spring",
      stiffness: 120,
      damping: 16,
    },
  },
} as const;

export function StatCard({ label, value, icon: Icon }: StatCardProps) {
  return (
    <motion.div
      className={styles.statCard}
      variants={itemVariants}
      whileHover={{
        y: -2,
        borderColor: "rgba(143, 174, 139, 0.2)",
        boxShadow: "0 8px 24px rgba(0, 0, 0, 0.15)",
      }}
    >
      <div className={styles.statLabel}>{label}</div>
      <div className={styles.statValue}>{value}</div>
      <div className={styles.statIcon}>
        <Icon size={20} />
      </div>
    </motion.div>
  );
}

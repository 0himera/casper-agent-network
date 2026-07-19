import type { LucideIcon } from "lucide-react";
import { motion } from "motion/react";
import { Skeleton } from "@/shared/ui";
import styles from "./Dashboard.module.css";

interface StatCardProps {
  label: string;
  value: string | number;
  icon: LucideIcon;
  isLoading?: boolean;
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

export function StatCard({ label, value, icon: Icon, isLoading }: StatCardProps) {
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
      {isLoading ? (
        <>
          <Skeleton width="55%" height={11} style={{ marginBottom: 12 }} />
          <Skeleton width="35%" height={28} />
          <Skeleton width={36} height={36} className={styles.statIconSkeleton} />
        </>
      ) : (
        <>
          <div className={styles.statLabel}>{label}</div>
          <div className={styles.statValue}>{value}</div>
          <div className={styles.statIcon}>
            <Icon size={20} aria-hidden="true" />
          </div>
        </>
      )}
    </motion.div>
  );
}

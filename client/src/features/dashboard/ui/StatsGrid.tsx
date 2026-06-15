import { STATS_CONFIG } from "@/features/dashboard/constants/stats";
import { StatCard } from "./StatCard";
import { motion } from "motion/react";
import styles from "./Dashboard.module.css";

const containerVariants = {
  hidden: { opacity: 0 },
  show: {
    opacity: 1,
    transition: {
      staggerChildren: 0.04,
    },
  },
};

export function StatsGrid() {
  return (
    <motion.div
      className={styles.statsGrid}
      variants={containerVariants}
      initial="hidden"
      animate="show"
    >
      {STATS_CONFIG.map((stat) => (
        <StatCard key={stat.label} {...stat} />
      ))}
    </motion.div>
  );
}

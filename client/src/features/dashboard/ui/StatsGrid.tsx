import { Bot, ListTodo, Coins, Star } from "lucide-react";
import { useMemo } from "react";
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

interface StatsGridProps {
  agentCount?: number;
  taskCount?: number;
  escrowedCSPR?: string;
  avgScore?: string;
}

export function StatsGrid({ agentCount, taskCount, escrowedCSPR, avgScore }: StatsGridProps) {
  const stats = useMemo(() => [
    { label: "Total Agents", value: agentCount ?? 0, icon: Bot },
    { label: "Total Tasks", value: taskCount ?? 0, icon: ListTodo },
    { label: "Escrowed CSPR", value: escrowedCSPR ?? "0 CSPR", icon: Coins },
    { label: "Avg Score", value: avgScore ?? "0", icon: Star },
  ], [agentCount, taskCount, escrowedCSPR, avgScore]);

  return (
    <motion.div
      className={styles.statsGrid}
      variants={containerVariants}
      initial="hidden"
      animate="show"
    >
      {stats.map((stat) => (
        <StatCard key={stat.label} {...stat} />
      ))}
    </motion.div>
  );
}

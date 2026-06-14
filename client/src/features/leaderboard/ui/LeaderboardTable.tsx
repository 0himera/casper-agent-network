import type { LeaderboardEntry } from "@/entities/reputation/types/types";
import { LeaderboardRow } from "./LeaderboardRow";
import { motion } from "motion/react";
import styles from "./Leaderboard.module.css";

interface LeaderboardTableProps {
  entries: LeaderboardEntry[];
  isLoading: boolean;
}

const containerVariants = {
  hidden: { opacity: 0 },
  show: {
    opacity: 1,
    transition: {
      staggerChildren: 0.03,
    },
  },
};

export function LeaderboardTable({ entries, isLoading }: LeaderboardTableProps) {
  if (isLoading) return <div className={styles.loading}>Loading leaderboard...</div>;

  return (
    <div className={styles.tableWrapper}>
      <table className={styles.table}>
        <thead>
          <tr>
            <th>Rank</th><th>Agent</th><th>Score</th><th>Tasks</th><th>Earnings</th>
          </tr>
        </thead>
        <motion.tbody
          variants={containerVariants}
          initial="hidden"
          animate="show"
        >
          {entries.map((e) => <LeaderboardRow key={e.agentPublicKey} entry={e} />)}
        </motion.tbody>
      </table>
    </div>
  );
}

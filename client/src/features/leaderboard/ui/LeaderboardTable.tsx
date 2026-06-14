import type { LeaderboardEntry } from "@/entities/reputation/types/types";
import { LeaderboardRow } from "./LeaderboardRow";
import styles from "./Leaderboard.module.css";

interface LeaderboardTableProps {
  entries: LeaderboardEntry[];
  isLoading: boolean;
}

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
        <tbody>
          {entries.map((e) => <LeaderboardRow key={e.agentPublicKey} entry={e} />)}
        </tbody>
      </table>
    </div>
  );
}

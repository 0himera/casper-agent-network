import { Medal } from "lucide-react";
import type { LeaderboardEntry } from "@/entities/reputation/types/types";
import { truncateAddress, stringToColor, getInitials } from "@/shared/utils/format";
import styles from "./Leaderboard.module.css";

function getRankIcon(r: number) {
  if (r === 1 || r === 2 || r === 3) {
    return <Medal size={16} style={{ display: "inline-block", verticalAlign: "middle" }} />;
  }
  return String(r);
}
function getRankClass(r: number) { return r === 1 ? styles.rankGold : r === 2 ? styles.rankSilver : r === 3 ? styles.rankBronze : ""; }

interface LeaderboardRowProps { entry: LeaderboardEntry }

export function LeaderboardRow({ entry }: LeaderboardRowProps) {
  return (
    <tr>
      <td className={`${styles.rankCell} ${getRankClass(entry.rank)}`}>{getRankIcon(entry.rank)}</td>
      <td>
        <div className={styles.agentCell}>
          <div className={styles.agentAvatar} style={{ background: stringToColor(entry.agentPublicKey) }}>
            {getInitials(entry.agentName)}
          </div>
          <div className={styles.agentNameCol}>
            <span className={styles.agentNameText}>{entry.agentName}</span>
            <span className={styles.agentKeyText}>{truncateAddress(entry.agentPublicKey)}</span>
          </div>
        </div>
      </td>
      <td className={styles.scoreCell}>{entry.score}</td>
      <td>{entry.tasksCompleted}</td>
      <td className={styles.earningsCell}>{entry.totalEarnings} CSPR</td>
    </tr>
  );
}

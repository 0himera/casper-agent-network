"use client";

import Link from "next/link";
import { Trophy, ArrowRight } from "lucide-react";
import type { LeaderboardEntry } from "@/entities/reputation/types/types";
import { getInitials, stringToColor, truncateAddress } from "@/shared/utils/format";
import { Skeleton } from "@/shared/ui";
import styles from "./Dashboard.module.css";

interface TopAgentsPanelProps {
  entries?: LeaderboardEntry[];
  isLoading?: boolean;
  limit?: number;
}

export function TopAgentsPanel({ entries, isLoading, limit = 3 }: TopAgentsPanelProps) {
  const top = entries?.slice(0, limit) ?? [];

  return (
    <div className={styles.dashboardSection}>
      <div className={styles.sectionHeader}>
        <h3 className={styles.sectionTitle} style={{ margin: 0 }}>
          <Trophy size={14} className={styles.actionIcon} aria-hidden="true" /> Top Agents
        </h3>
        <Link href="/leaderboard" className={styles.sectionLink}>
          Leaderboard <ArrowRight size={12} aria-hidden="true" />
        </Link>
      </div>

      <div className={styles.topAgentsList}>
        {isLoading ? (
          Array.from({ length: limit }).map((_, i) => (
            <div key={i} className={styles.topAgentRow}>
              <Skeleton width={24} height={14} />
              <Skeleton width={32} height={32} borderRadius="var(--radius-sm)" />
              <div className={styles.topAgentInfo}>
                <Skeleton width="55%" height={13} style={{ marginBottom: 6 }} />
                <Skeleton width="35%" height={10} />
              </div>
              <Skeleton width={40} height={16} />
            </div>
          ))
        ) : top.length > 0 ? (
          top.map((entry) => (
            <Link
              key={entry.agentPublicKey}
              href={`/agents/${entry.agentPublicKey}`}
              className={styles.topAgentRow}
            >
              <span className={styles.topAgentRank}>#{entry.rank}</span>
              <div
                className={styles.topAgentAvatar}
                style={{ background: stringToColor(entry.agentPublicKey) }}
              >
                {getInitials(entry.agentName)}
              </div>
              <div className={styles.topAgentInfo}>
                <span className={styles.topAgentName}>{entry.agentName}</span>
                <span className={styles.topAgentKey}>
                  {truncateAddress(entry.agentPublicKey, 6, 4)}
                </span>
              </div>
              <span className={styles.topAgentScore}>{entry.score}</span>
            </Link>
          ))
        ) : (
          <div className={styles.emptyStateText}>No rankings available.</div>
        )}
      </div>
    </div>
  );
}

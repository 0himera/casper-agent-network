"use client";

import { useState } from "react";
import { useLeaderboardQuery } from "@/features/leaderboard/api/queries";
import type { LeaderboardDomain } from "@/entities/reputation/types/types";
import { DomainTabs } from "@/features/leaderboard/ui/DomainTabs";
import { LeaderboardTable } from "@/features/leaderboard/ui/LeaderboardTable";
import styles from "@/features/leaderboard/ui/Leaderboard.module.css";

export default function LeaderboardPage() {
  const [domain, setDomain] = useState<LeaderboardDomain>("global");
  const { data: entries = [], isLoading } = useLeaderboardQuery(domain);

  return (
    <div className={styles.page}>
      <div className={styles.header}>
        <h1 className={styles.title}>Leaderboard</h1>
        <p className={styles.subtitle}>Top-performing agents ranked by on-chain reputation score</p>
      </div>
      <DomainTabs active={domain} onChange={setDomain} />
      <LeaderboardTable entries={entries} isLoading={isLoading} />
    </div>
  );
}

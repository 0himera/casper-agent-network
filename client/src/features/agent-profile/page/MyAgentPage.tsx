"use client";

import Link from "next/link";
import { Bot } from "lucide-react";
import { MOCK_AGENTS } from "@/shared/api/mock-data";
import { useAppStore } from "@/shared/providers/AppStoreProvider";
import { truncateAddress } from "@/shared/utils/format";
import { MyAgentStats } from "@/features/agent-profile/ui/MyAgentStats";
import { PriceConfig } from "@/features/agent-profile/ui/PriceConfig";
import { BenchmarkPanel } from "@/features/agent-profile/ui/BenchmarkPanel";
import styles from "@/features/agent-profile/ui/MyAgent.module.css";

export default function MyAgentPage() {
  const walletAddress = useAppStore((s) => s.walletAddress);
  const agent = walletAddress ? MOCK_AGENTS[0] : null;

  if (!agent) {
    return (
      <div className={styles.page}>
        <div className={styles.emptyState}>
          <Bot size={64} style={{ opacity: 0.3, color: "var(--text-muted)" }} />
          <div className={styles.emptyTitle}>No Agent Registered</div>
          <div className={styles.emptyDescription}>Connect your wallet and register a bot.</div>
          <Link href="/register" className={styles.registerLink}>Register Bot</Link>
        </div>
      </div>
    );
  }

  return (
    <div className={styles.page}>
      <h1 className={styles.title}>My Agent Dashboard</h1>
      <div className={styles.statusRow}>
        <div className={styles.statusDot} />
        <span className={styles.statusLabel}>Active</span>
        <span className={styles.publicKey}>{truncateAddress(agent.publicKey, 12, 8)}</span>
      </div>
      <MyAgentStats agent={agent} />
      <PriceConfig agent={agent} />
      <BenchmarkPanel />
    </div>
  );
}

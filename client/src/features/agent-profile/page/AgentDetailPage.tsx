"use client";

import { use } from "react";
import Link from "next/link";
import { ArrowLeft } from "lucide-react";
import { useAgentByKeyQuery } from "@/features/agents/api/queries";
import { AgentHero } from "@/features/agent-profile/ui/AgentHero";
import { AgentStatsRow } from "@/features/agent-profile/ui/AgentStatsRow";
import { SkillBars } from "@/features/agent-profile/ui/SkillBars";
import { AgentStakingPanel } from "@/features/agent-profile/ui/AgentStakingPanel";
import { AgentTechInfo } from "@/features/agent-profile/ui/AgentTechInfo";
import { BenchmarkPanel } from "@/features/agent-profile/ui/BenchmarkPanel";
import { useAppStore } from "@/shared/providers/AppStoreProvider";
import { SkeletonDetail } from "@/shared/ui";
import { motion } from "motion/react";
import styles from "@/features/agent-profile/ui/AgentDetail.module.css";

const containerVariants = {
  hidden: { opacity: 0 },
  show: {
    opacity: 1,
    transition: {
      staggerChildren: 0.08,
    },
  },
} as const;

const itemVariants = {
  hidden: { opacity: 0, y: 15 },
  show: {
    opacity: 1,
    y: 0,
    transition: {
      type: "spring",
      stiffness: 100,
      damping: 15,
    },
  },
} as const;

export default function AgentDetailPage({ params }: { params: Promise<{ agentId: string }> }) {
  const { agentId } = use(params);
  const { data: agent, isLoading } = useAgentByKeyQuery(agentId);
  const walletAddress = useAppStore((s) => s.walletAddress);

  if (isLoading) {
    return (
      <div className={styles.page}>
        <SkeletonDetail />
      </div>
    );
  }
  if (!agent) return <div className={styles.loading}>Agent not found</div>;

  const isOwner =
    walletAddress && agent.publicKey && walletAddress.toLowerCase() === agent.publicKey.toLowerCase();

  return (
    <motion.div
      className={styles.page}
      variants={containerVariants}
      initial="hidden"
      animate="show"
    >
      <motion.div variants={itemVariants}>
        <Link href="/dashboard" className={styles.backLink}><ArrowLeft size={16} /> Back to Dashboard</Link>
      </motion.div>
      <motion.div variants={itemVariants}>
        <AgentHero agent={agent} />
      </motion.div>
      <motion.div variants={itemVariants}>
        <AgentStatsRow agent={agent} />
      </motion.div>
      {isOwner && (
        <motion.div variants={itemVariants}>
          <AgentStakingPanel agent={agent} />
        </motion.div>
      )}
      <motion.div variants={itemVariants}>
        <SkillBars agent={agent} />
      </motion.div>
      <motion.div variants={itemVariants}>
        <BenchmarkPanel publicKey={agent.publicKey} />
      </motion.div>
      <motion.div variants={itemVariants}>
        <AgentTechInfo agent={agent} />
      </motion.div>
    </motion.div>
  );
}

"use client";

import { use } from "react";
import Link from "next/link";
import { ArrowLeft } from "lucide-react";
import { useAgentByKeyQuery } from "@/features/agents/api/queries";
import { AgentHero } from "@/features/agent-profile/ui/AgentHero";
import { AgentStatsRow } from "@/features/agent-profile/ui/AgentStatsRow";
import { SkillBars } from "@/features/agent-profile/ui/SkillBars";
import { AgentTechInfo } from "@/features/agent-profile/ui/AgentTechInfo";
import styles from "@/features/agent-profile/ui/AgentDetail.module.css";

export default function AgentDetailPage({ params }: { params: Promise<{ agentId: string }> }) {
  const { agentId } = use(params);
  const { data: agent, isLoading } = useAgentByKeyQuery(agentId);

  if (isLoading) return <div className={styles.loading}>Loading agent...</div>;
  if (!agent) return <div className={styles.loading}>Agent not found</div>;

  return (
    <div className={styles.page}>
      <Link href="/dashboard" className={styles.backLink}><ArrowLeft size={16} /> Back to Dashboard</Link>
      <AgentHero agent={agent} />
      <AgentStatsRow agent={agent} />
      <SkillBars agent={agent} />
      <AgentTechInfo agent={agent} />
    </div>
  );
}

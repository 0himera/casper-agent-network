"use client";

import { useMemo, useState } from "react";
import { Bot } from "lucide-react";
import { useAgentsQuery } from "@/features/agents/api/queries";
import { useTasksQuery } from "@/features/tasks/api/queries";
import { useLeaderboardQuery } from "@/features/leaderboard/api/queries";
import type { AgentSkill, AgentStatus } from "@/entities/agent/types/types";
import { StatsGrid } from "@/features/dashboard/ui/StatsGrid";
import { AgentsToolbar } from "@/features/dashboard/ui/AgentsToolbar";
import { AgentCard } from "@/features/dashboard/ui/AgentCard";
import { SkeletonCardGrid } from "@/shared/ui";
import { motion } from "motion/react";
import styles from "@/features/dashboard/ui/Dashboard.module.css";

const containerVariants = {
  hidden: { opacity: 0 },
  show: {
    opacity: 1,
    transition: {
      staggerChildren: 0.05,
    },
  },
};

export default function DashboardPage() {
  const [search, setSearch] = useState("");
  const [skillFilter, setSkillFilter] = useState<AgentSkill | "">("");
  const [statusFilter, setStatusFilter] = useState<AgentStatus | "">("");

  const { data: agents, isLoading } = useAgentsQuery({
    search: search || undefined,
    skill: skillFilter || undefined,
    status: statusFilter || undefined,
  });
  const { data: totalAgents } = useAgentsQuery({});
  const { data: totalTasks } = useTasksQuery();
  const { data: leaderboard } = useLeaderboardQuery();

  const escrowedCSPR = useMemo(() => {
    if (!totalTasks?.length) return "0 CSPR";
    const total = totalTasks
      .filter((t) => t.status === "open" || t.status === "in_progress")
      .reduce((sum, t) => sum + t.budget, 0);
    return `${total.toFixed(1)} CSPR`;
  }, [totalTasks]);

  const avgScore = useMemo(() => {
    if (!leaderboard?.length) return "0";
    const total = leaderboard.reduce((sum, e) => sum + e.score, 0);
    return (total / leaderboard.length).toFixed(1);
  }, [leaderboard]);

  return (
    <div className={styles.page}>
      <StatsGrid agentCount={totalAgents?.length} taskCount={totalTasks?.length} escrowedCSPR={escrowedCSPR} avgScore={avgScore} />
      <AgentsToolbar
        search={search} onSearchChange={setSearch}
        skillFilter={skillFilter} onSkillChange={setSkillFilter}
        statusFilter={statusFilter} onStatusChange={setStatusFilter}
      />
      {isLoading ? (
        <SkeletonCardGrid count={6} />
      ) : agents && agents.length > 0 ? (
        <motion.div
          className={styles.agentsGrid}
          variants={containerVariants}
          initial="hidden"
          animate="show"
          key={`${search}-${skillFilter}-${statusFilter}`}
        >
          {agents.map((a) => <AgentCard key={a.publicKey} agent={a} />)}
        </motion.div>
      ) : (
        <div className={styles.emptyState}>
          <Bot className={styles.emptyIcon} />
          <div className={styles.emptyTitle}>No agents found</div>
          <div className={styles.emptyDescription}>Try adjusting your filters</div>
        </div>
      )}
    </div>
  );
}

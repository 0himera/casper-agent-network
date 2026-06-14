"use client";

import { useState } from "react";
import { Bot } from "lucide-react";
import { useAgentsQuery } from "@/features/agents/api/queries";
import type { AgentSkill, AgentStatus } from "@/entities/agent/types/types";
import { StatsGrid } from "@/features/dashboard/ui/StatsGrid";
import { AgentsToolbar } from "@/features/dashboard/ui/AgentsToolbar";
import { AgentCard } from "@/features/dashboard/ui/AgentCard";
import styles from "@/features/dashboard/ui/Dashboard.module.css";

export default function DashboardPage() {
  const [search, setSearch] = useState("");
  const [skillFilter, setSkillFilter] = useState<AgentSkill | "">("");
  const [statusFilter, setStatusFilter] = useState<AgentStatus | "">("");

  const { data: agents, isLoading } = useAgentsQuery({
    search: search || undefined,
    skill: skillFilter || undefined,
    status: statusFilter || undefined,
  });

  return (
    <div className={styles.page}>
      <StatsGrid />
      <AgentsToolbar
        search={search} onSearchChange={setSearch}
        skillFilter={skillFilter} onSkillChange={setSkillFilter}
        statusFilter={statusFilter} onStatusChange={setStatusFilter}
      />
      {isLoading ? (
        <div className={styles.loading}>Loading agents...</div>
      ) : agents && agents.length > 0 ? (
        <div className={styles.agentsGrid}>
          {agents.map((a) => <AgentCard key={a.publicKey} agent={a} />)}
        </div>
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

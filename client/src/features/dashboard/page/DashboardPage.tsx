"use client";

import { useMemo } from "react";
import { Trophy, PlusCircle, Activity, UserCog } from "lucide-react";
import { useAgentsQuery } from "@/features/agents/api/queries";
import { useTasksQuery } from "@/features/tasks/api/queries";
import { useLeaderboardQuery } from "@/features/leaderboard/api/queries";
import { StatsGrid } from "@/features/dashboard/ui/StatsGrid";
import { TaskVolumeChart } from "@/features/dashboard/ui/TaskVolumeChart";
import { RecentTasks } from "@/features/dashboard/ui/RecentTasks";
import { NetworkStatus } from "@/features/dashboard/ui/NetworkStatus";
import { TopAgentsPanel } from "@/features/dashboard/ui/TopAgentsPanel";
import { HostedAgentDialog } from "@/features/dashboard/ui/HostedAgentDialog";
import { useAppStore } from "@/shared/providers/AppStoreProvider";
import { formatCSPR } from "@/shared/utils/format";
import { motion } from "motion/react";
import Link from "next/link";
import styles from "@/features/dashboard/ui/Dashboard.module.css";

const itemVariants = {
  hidden: { opacity: 0, y: 10 },
  show: { opacity: 1, y: 0, transition: { type: "spring", stiffness: 100 } as const },
};

export default function DashboardPage() {
  const walletAddress = useAppStore((s) => s.walletAddress);
  const { data: totalAgents, isLoading: agentsLoading, isError: agentsError } = useAgentsQuery({});
  const { data: totalTasks, isLoading: tasksLoading, isError: tasksError } = useTasksQuery();
  const {
    data: leaderboard,
    isLoading: leaderboardLoading,
    isError: leaderboardError,
  } = useLeaderboardQuery();

  const isLoading = agentsLoading || tasksLoading || leaderboardLoading;
  const isError = agentsError || tasksError || leaderboardError;
  const dataHealthy = !agentsError && !tasksError && !leaderboardError;

  const escrowedCSPR = useMemo(() => {
    if (!totalTasks?.length) return "0 CSPR";
    const total = totalTasks
      .filter((t) => t.status === "open" || t.status === "in_progress")
      .reduce((sum, t) => sum + t.budget, 0);
    return `${formatCSPR(total)}`;
  }, [totalTasks]);

  const avgScore = useMemo(() => {
    if (!leaderboard?.length) return "0";
    const total = leaderboard.reduce((sum, e) => sum + e.score, 0);
    return (total / leaderboard.length).toFixed(1);
  }, [leaderboard]);

  const quickActions = useMemo(() => {
    const actions: Array<{
      href: string;
      icon: typeof PlusCircle;
      title: string;
      desc: string;
    }> = [
      {
        href: "/tasks/create",
        icon: PlusCircle,
        title: "Create a Task",
        desc: "Hire AI agents for analysis",
      },
      {
        href: "/leaderboard",
        icon: Trophy,
        title: "View Leaderboard",
        desc: "Audit top-performing nodes",
      },
    ];
    if (walletAddress) {
      actions.push({
        href: "/my-agent",
        icon: UserCog,
        title: "My Agent",
        desc: "Manage your on-chain agent",
      });
    }
    return actions;
  }, [walletAddress]);

  return (
    <div className={styles.page}>
      <motion.div
        className={styles.pageHeader}
        initial={{ opacity: 0, y: 8 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.3 }}
      >
        <h1 className={styles.pageTitle}>Dashboard Overview</h1>
        <p className={styles.pageSubtitle}>
          Live network activity, recent jobs, and agent performance.
        </p>
      </motion.div>

      {isError && (
        <div className={styles.errorBanner} role="alert" aria-live="polite">
          Some dashboard data failed to load. The numbers shown may be incomplete.
        </div>
      )}

      <StatsGrid
        agentCount={totalAgents?.length}
        taskCount={totalTasks?.length}
        escrowedCSPR={escrowedCSPR}
        avgScore={avgScore}
        isLoading={isLoading}
      />

      <div className={styles.dashboardLayout}>
        <div className={styles.dashboardColumn}>
          <motion.div variants={itemVariants} initial="hidden" animate="show">
            <TaskVolumeChart tasks={totalTasks} isLoading={tasksLoading} />
          </motion.div>

          <motion.div variants={itemVariants} initial="hidden" animate="show">
            <RecentTasks tasks={totalTasks} isLoading={tasksLoading} limit={5} />
          </motion.div>
        </div>

        <div className={styles.dashboardColumn}>
          <motion.div variants={itemVariants} initial="hidden" animate="show">
            <div className={styles.dashboardSection}>
              <h3 className={styles.sectionTitle}>
                <Activity size={14} className={styles.actionIcon} aria-hidden="true" /> Quick
                Actions
              </h3>
              <div className={styles.quickActions}>
                <HostedAgentDialog />
                {quickActions.map((a) => (
                  <Link key={a.href} href={a.href} className={styles.actionCard}>
                    <a.icon size={18} className={styles.actionIcon} aria-hidden="true" />
                    <div className={styles.actionContent}>
                      <span className={styles.actionTitle}>{a.title}</span>
                      <span className={styles.actionDesc}>{a.desc}</span>
                    </div>
                  </Link>
                ))}
              </div>
            </div>
          </motion.div>

          <motion.div variants={itemVariants} initial="hidden" animate="show">
            <TopAgentsPanel entries={leaderboard} isLoading={leaderboardLoading} limit={3} />
          </motion.div>

          <motion.div variants={itemVariants} initial="hidden" animate="show">
            <NetworkStatus dataHealthy={dataHealthy} />
          </motion.div>
        </div>
      </div>
    </div>
  );
}

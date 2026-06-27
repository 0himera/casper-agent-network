"use client";

import { useMemo } from "react";
import Link from "next/link";
import {
  ListTodo,
  Bot,
  Trophy,
  PlusCircle,
  Activity,
  ShieldCheck,
  Cpu,
  ArrowRight,
} from "lucide-react";
import { useAgentsQuery } from "@/features/agents/api/queries";
import { useTasksQuery } from "@/features/tasks/api/queries";
import { useLeaderboardQuery } from "@/features/leaderboard/api/queries";
import { StatsGrid } from "@/features/dashboard/ui/StatsGrid";
import {
  AreaChart,
  Area,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
} from "recharts";
import { motion } from "motion/react";
import styles from "@/features/dashboard/ui/Dashboard.module.css";

const chartData = [
  { day: "Mon", tasks: 5, volume: 25 },
  { day: "Tue", tasks: 8, volume: 40 },
  { day: "Wed", tasks: 12, volume: 60 },
  { day: "Thu", tasks: 9, volume: 45 },
  { day: "Fri", tasks: 17, volume: 85 },
  { day: "Sat", tasks: 22, volume: 110 },
  { day: "Sun", tasks: 28, volume: 140 },
];

const itemVariants = {
  hidden: { opacity: 0, y: 10 },
  show: { opacity: 1, y: 0, transition: { type: "spring", stiffness: 100 } },
};

export default function DashboardPage() {
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

  const recentTasks = useMemo(() => {
    if (!totalTasks) return [];
    return [...totalTasks]
      .sort((a, b) => new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime())
      .slice(0, 3);
  }, [totalTasks]);

  return (
    <div className={styles.page}>
      <h1 style={{ fontSize: "22px", fontWeight: 700, color: "var(--text-primary)", marginBottom: "var(--space-xs)" }}>
        Dashboard Overview
      </h1>
      
      <StatsGrid
        agentCount={totalAgents?.length}
        taskCount={totalTasks?.length}
        escrowedCSPR={escrowedCSPR}
        avgScore={avgScore}
      />

      <div className={styles.dashboardLayout}>
        {/* Left Side: Activity Chart & Recent Tasks */}
        <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-xl)" }}>
          {/* Chart */}
          <div className={styles.dashboardSection}>
            <h3 className={styles.sectionTitle}>
              <Activity size={14} className={styles.actionIcon} /> Network Task Volume
            </h3>
            <div style={{ width: "100%", height: 200, marginTop: "10px" }}>
              <ResponsiveContainer width="100%" height="100%">
                <AreaChart data={chartData} margin={{ top: 10, right: 10, left: -20, bottom: 0 }}>
                  <defs>
                    <linearGradient id="colorTasks" x1="0" y1="0" x2="0" y2="1">
                      <stop offset="5%" stopColor="var(--accent-primary, #6366f1)" stopOpacity={0.4}/>
                      <stop offset="95%" stopColor="var(--accent-primary, #6366f1)" stopOpacity={0}/>
                    </linearGradient>
                  </defs>
                  <XAxis dataKey="day" stroke="var(--text-muted)" fontSize={11} tickLine={false} />
                  <YAxis stroke="var(--text-muted)" fontSize={11} tickLine={false} />
                  <Tooltip
                    contentStyle={{
                      background: "var(--bg-card-solid, #141821)",
                      borderColor: "var(--border-color)",
                      borderRadius: "6px",
                      fontSize: "12px",
                      color: "#fff"
                    }}
                  />
                  <Area
                    type="monotone"
                    dataKey="tasks"
                    name="Completed Tasks"
                    stroke="var(--accent-primary, #6366f1)"
                    strokeWidth={2}
                    fillOpacity={1}
                    fill="url(#colorTasks)"
                  />
                </AreaChart>
              </ResponsiveContainer>
            </div>
          </div>

          {/* Recent Tasks Feed */}
          <div className={styles.dashboardSection}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <h3 className={styles.sectionTitle} style={{ margin: 0 }}>
                <ListTodo size={14} className={styles.actionIcon} /> Recent Job Activity
              </h3>
              <Link href="/tasks" style={{ fontSize: "11px", color: "var(--accent-cyan, #00f2fe)", textDecoration: "none", display: "flex", alignItems: "center", gap: "4px" }}>
                All Jobs <ArrowRight size={12} />
              </Link>
            </div>
            <div style={{ display: "flex", flexDirection: "column", gap: "10px", marginTop: "10px" }}>
              {recentTasks.length > 0 ? (
                recentTasks.map((t) => (
                  <div key={t.id} className={styles.taskItem}>
                    <div className={styles.taskItemLeft}>
                      <div className={styles.taskPrompt}>{t.prompt}</div>
                      <div className={styles.taskMeta}>
                        <span className={styles.taskBudget}>{t.budget} CSPR</span>
                        <span>•</span>
                        <span style={{ textTransform: "capitalize" }}>{t.status.replace("_", " ")}</span>
                        <span>•</span>
                        <span>{t.id}</span>
                      </div>
                    </div>
                    <Link href={`/tasks/${t.id}`} className={styles.viewTaskButton}>
                      View
                    </Link>
                  </div>
                ))
              ) : (
                <div style={{ textAlign: "center", color: "var(--text-muted)", fontSize: "12px", padding: "20px 0" }}>
                  No tasks recorded yet.
                </div>
              )}
            </div>
          </div>
        </div>

        {/* Right Side: Quick Actions & Status */}
        <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-xl)" }}>
          {/* Quick Actions */}
          <div className={styles.dashboardSection}>
            <h3 className={styles.sectionTitle}>Quick Actions</h3>
            <div className={styles.quickActions}>
              <Link href="/tasks/create" className={styles.actionCard}>
                <PlusCircle size={18} className={styles.actionIcon} />
                <div className={styles.actionContent}>
                  <span className={styles.actionTitle}>Create a Task</span>
                  <span className={styles.actionDesc}>Hire AI agents for analysis</span>
                </div>
              </Link>
              <Link href="/register" className={styles.actionCard}>
                <Bot size={18} className={styles.actionIcon} />
                <div className={styles.actionContent}>
                  <span className={styles.actionTitle}>Register a Bot</span>
                  <span className={styles.actionDesc}>Connect new LLM or autonomous key</span>
                </div>
              </Link>
              <Link href="/leaderboard" className={styles.actionCard}>
                <Trophy size={18} className={styles.actionIcon} />
                <div className={styles.actionContent}>
                  <span className={styles.actionTitle}>View Leaderboard</span>
                  <span className={styles.actionDesc}>Audit top-performing nodes</span>
                </div>
              </Link>
            </div>
          </div>

          {/* Platform Status */}
          <div className={styles.dashboardSection}>
            <h3 className={styles.sectionTitle}>Network Status</h3>
            <div className={styles.systemStatusGrid}>
              <div className={styles.systemStatusCard}>
                <div className={styles.statusDotPulse} />
                <span>Casper Node</span>
              </div>
              <div className={styles.systemStatusCard}>
                <Cpu size={14} style={{ color: "#10b981" }} />
                <span>Registry API</span>
              </div>
              <div className={styles.systemStatusCard}>
                <ShieldCheck size={14} style={{ color: "#10b981" }} />
                <span>Judge Online</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

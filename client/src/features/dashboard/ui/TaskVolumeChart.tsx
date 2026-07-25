"use client";

import { useMemo } from "react";
import {
  AreaChart,
  Area,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
  CartesianGrid,
} from "recharts";
import { Activity } from "lucide-react";
import type { TaskEntity } from "@/entities/task/types/types";
import { Skeleton } from "@/shared/ui";
import styles from "./Dashboard.module.css";

interface TaskVolumeChartProps {
  tasks?: TaskEntity[];
  isLoading?: boolean;
}

function buildChartData(tasks: TaskEntity[] = [], days = 7) {
  const today = new Date();
  today.setHours(0, 0, 0, 0);
  const data: { date: string; label: string; tasks: number; volume: number }[] = [];
  const map = new Map<string, (typeof data)[number]>();

  for (let i = days - 1; i >= 0; i--) {
    const d = new Date(today);
    d.setDate(d.getDate() - i);
    const iso = d.toISOString().split("T")[0];
    const label = d.toLocaleDateString("en-US", { weekday: "short" });
    const entry = { date: iso, label, tasks: 0, volume: 0 };
    data.push(entry);
    map.set(iso, entry);
  }

  for (const t of tasks) {
    const iso = new Date(t.createdAt).toISOString().split("T")[0];
    const entry = map.get(iso);
    if (entry) {
      entry.tasks += 1;
      if (t.status === "completed") {
        entry.volume += t.budget;
      }
    }
  }

  return data;
}

export function TaskVolumeChart({ tasks, isLoading }: TaskVolumeChartProps) {
  const data = useMemo(() => buildChartData(tasks), [tasks]);
  const hasActivity = data.some((d) => d.tasks > 0);

  return (
    <div className={styles.dashboardSection}>
      <h3 className={styles.sectionTitle}>
        <Activity size={14} className={styles.actionIcon} aria-hidden="true" /> Network Task Volume
      </h3>
      {isLoading ? (
        <div style={{ width: "100%", height: 200, marginTop: 10 }}>
          <Skeleton width="100%" height="100%" borderRadius="var(--radius-md)" />
        </div>
      ) : !hasActivity ? (
        <div className={styles.chartEmpty}>No task activity in the last 7 days.</div>
      ) : (
        <div style={{ width: "100%", height: 200, marginTop: 10, minWidth: 0 }}>
          <ResponsiveContainer width="100%" height="100%" minWidth={0} minHeight={200}>
            <AreaChart data={data} margin={{ top: 10, right: 10, left: -20, bottom: 0 }}>
              <defs>
                <linearGradient id="colorTasks" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor="var(--accent-primary, #6366f1)" stopOpacity={0.4} />
                  <stop offset="95%" stopColor="var(--accent-primary, #6366f1)" stopOpacity={0} />
                </linearGradient>
                <linearGradient id="colorVolume" x1="0" y1="0" x2="0" y2="1">
                  <stop
                    offset="5%"
                    stopColor="var(--accent-secondary, #d4a07a)"
                    stopOpacity={0.3}
                  />
                  <stop offset="95%" stopColor="var(--accent-secondary, #d4a07a)" stopOpacity={0} />
                </linearGradient>
              </defs>
              <CartesianGrid strokeDasharray="3 3" stroke="var(--border-color)" vertical={false} />
              <XAxis dataKey="label" stroke="var(--text-muted)" fontSize={11} tickLine={false} />
              <YAxis yAxisId="left" stroke="var(--text-muted)" fontSize={11} tickLine={false} />
              <YAxis
                yAxisId="right"
                orientation="right"
                stroke="var(--text-muted)"
                fontSize={11}
                tickLine={false}
              />
              <Tooltip
                contentStyle={{
                  background: "var(--bg-card-solid, #141821)",
                  borderColor: "var(--border-color)",
                  borderRadius: "6px",
                  fontSize: "12px",
                  color: "#fff",
                }}
              />
              <Area
                yAxisId="left"
                type="monotone"
                dataKey="tasks"
                name="Created Tasks"
                stroke="var(--accent-primary, #6366f1)"
                strokeWidth={2}
                fillOpacity={1}
                fill="url(#colorTasks)"
              />
              <Area
                yAxisId="right"
                type="monotone"
                dataKey="volume"
                name="Completed Volume (CSPR)"
                stroke="var(--accent-secondary, #d4a07a)"
                strokeWidth={2}
                fillOpacity={1}
                fill="url(#colorVolume)"
              />
            </AreaChart>
          </ResponsiveContainer>
        </div>
      )}
    </div>
  );
}

"use client";

import { useMemo } from "react";
import Link from "next/link";
import { ListTodo, ArrowRight, Clock, Hash } from "lucide-react";
import type { TaskEntity } from "@/entities/task/types/types";
import { TaskStatusBadge } from "@/entities/task/ui/TaskStatusBadge";
import { SKILL_LABELS } from "@/entities/agent/types/types";
import { formatTimeAgo } from "@/shared/utils/format";
import { Skeleton } from "@/shared/ui";
import styles from "./Dashboard.module.css";

interface RecentTasksProps {
  tasks?: TaskEntity[];
  isLoading?: boolean;
  limit?: number;
}

export function RecentTasks({ tasks, isLoading, limit = 5 }: RecentTasksProps) {
  const recentTasks = useMemo(() => {
    if (!tasks) return [];
    return [...tasks]
      .sort((a, b) => new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime())
      .slice(0, limit);
  }, [tasks, limit]);

  return (
    <div className={styles.dashboardSection}>
      <div className={styles.sectionHeader}>
        <h3 className={styles.sectionTitle} style={{ margin: 0 }}>
          <ListTodo size={14} className={styles.actionIcon} aria-hidden="true" /> Recent Job
          Activity
        </h3>
        <Link href="/tasks" className={styles.sectionLink}>
          All Jobs <ArrowRight size={12} aria-hidden="true" />
        </Link>
      </div>

      <div className={styles.recentTasksList}>
        {isLoading ? (
          Array.from({ length: 3 }).map((_, i) => (
            <div key={i} className={styles.taskItem}>
              <div className={styles.taskItemLeft}>
                <Skeleton width="70%" height={13} style={{ marginBottom: 8 }} />
                <Skeleton width="40%" height={11} />
              </div>
              <Skeleton width={44} height={24} borderRadius="var(--radius-md)" />
            </div>
          ))
        ) : recentTasks.length > 0 ? (
          recentTasks.map((t) => (
            <Link key={t.id} href={`/tasks/${t.id}`} className={styles.taskItem}>
              <div className={styles.taskItemLeft}>
                <div className={styles.taskPrompt} title={t.prompt}>
                  {t.prompt}
                </div>
                <div className={styles.taskMeta}>
                  <span className={styles.taskBudget}>{t.budget} CSPR</span>
                  <span className={styles.taskDot} />
                  <TaskStatusBadge status={t.status} />
                  <span className={styles.taskDot} />
                  <span className={styles.taskDomain}>{SKILL_LABELS[t.domain]}</span>
                  <span className={styles.taskDot} />
                  <span className={styles.taskTime}>
                    <Clock size={10} aria-hidden="true" /> {formatTimeAgo(t.createdAt)}
                  </span>
                  <span className={styles.taskDot} />
                  <span className={styles.taskIdCompact}>
                    <Hash size={10} aria-hidden="true" /> {t.id}
                  </span>
                </div>
              </div>
              <span className={styles.viewTaskButton}>View</span>
            </Link>
          ))
        ) : (
          <div className={styles.emptyStateText}>No tasks recorded yet.</div>
        )}
      </div>
    </div>
  );
}

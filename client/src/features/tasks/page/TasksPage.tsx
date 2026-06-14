"use client";

import { useMemo, useState } from "react";
import Link from "next/link";
import { Plus, ListTodo } from "lucide-react";
import { useTasksQuery } from "@/features/tasks/api/queries";
import type { TaskStatus } from "@/entities/task/types/types";
import { StatusTabs } from "@/features/tasks/ui/StatusTabs";
import { TaskCard } from "@/features/tasks/ui/TaskCard";
import styles from "@/features/tasks/ui/Tasks.module.css";

export default function TasksPage() {
  const [filter, setFilter] = useState<TaskStatus | "all">("all");
  const { data: allTasks = [], isLoading } = useTasksQuery();

  const filtered = useMemo(() => {
    return filter === "all" ? allTasks : allTasks.filter((t) => t.status === filter);
  }, [allTasks, filter]);

  const counts = useMemo(() => {
    const c: Record<string, number> = { all: allTasks.length };
    for (const t of allTasks) c[t.status] = (c[t.status] ?? 0) + 1;
    return c;
  }, [allTasks]);

  return (
    <div className={styles.page}>
      <div className={styles.header}>
        <div className={styles.headerLeft}>
          <h1 className={styles.title}>Job Board</h1>
          <p className={styles.subtitle}>Browse and manage tasks on the network</p>
        </div>
        <Link href="/tasks/create" className={styles.createButton}><Plus size={16} /> Create Task</Link>
      </div>
      <StatusTabs active={filter} onChange={setFilter} counts={counts} />
      {isLoading ? (
        <div className={styles.loading}>Loading tasks...</div>
      ) : filtered.length > 0 ? (
        <div className={styles.tasksGrid}>
          {filtered.map((t) => <TaskCard key={t.id} task={t} />)}
        </div>
      ) : (
        <div className={styles.emptyState}>
          <ListTodo className={styles.emptyIcon} />
          <div className={styles.emptyTitle}>No tasks found</div>
          <div className={styles.emptyDescription}>No tasks match this filter</div>
        </div>
      )}
    </div>
  );
}

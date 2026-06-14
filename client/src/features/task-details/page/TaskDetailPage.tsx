"use client";

import { use } from "react";
import Link from "next/link";
import { ArrowLeft, ClipboardList, Coins, Hash } from "lucide-react";
import { useTaskByIdQuery } from "@/features/tasks/api/queries";
import { SKILL_LABELS } from "@/entities/agent/types/types";
import { formatTimeAgo } from "@/shared/utils/format";
import { StatusTimeline } from "@/features/task-details/ui/StatusTimeline";
import { EvaluationPanel } from "@/features/task-details/ui/EvaluationPanel";
import { TransactionsList } from "@/features/task-details/ui/TransactionsList";
import styles from "@/features/task-details/ui/TaskDetail.module.css";

export default function TaskDetailPage({ params }: { params: Promise<{ taskId: string }> }) {
  const { taskId } = use(params);
  const { data: task, isLoading } = useTaskByIdQuery(taskId);

  if (isLoading) return <div className={styles.loading}>Loading task...</div>;
  if (!task) return <div className={styles.loading}>Task not found</div>;

  const steps = [
    { label: "Created", time: formatTimeAgo(task.createdAt) },
    { label: "Assigned", time: task.assignedAgentName ? formatTimeAgo(task.createdAt) : null, detail: task.assignedAgentName ?? undefined },
    { label: "In Progress", time: task.status !== "open" ? formatTimeAgo(task.updatedAt) : null },
    { label: "Submitted", time: task.result ? formatTimeAgo(task.updatedAt) : null },
    { label: "Completed", time: task.status === "completed" ? formatTimeAgo(task.updatedAt) : null, detail: task.status === "completed" ? "Escrow released" : undefined },
  ];

  return (
    <div className={styles.page}>
      <Link href="/tasks" className={styles.backLink}><ArrowLeft size={16} /> Back to Job Board</Link>
      <div className={styles.header}>
        <div className={styles.titleRow}><h1 className={styles.title}>{task.prompt.slice(0, 60)}...</h1></div>
        <div className={styles.meta}>
          <span className={styles.metaItem}><ClipboardList size={13} /> {SKILL_LABELS[task.domain]}</span>
          <span className={styles.metaItem}><Coins size={13} /> {task.budget} CSPR</span>
          <span className={styles.metaItem}><Hash size={13} /> {task.id}</span>
        </div>
      </div>
      <StatusTimeline steps={steps} />
      {task.result && (
        <div className={styles.section}>
          <h3 className={styles.sectionTitle}>Result</h3>
          <div className={styles.resultContent}>{task.result}</div>
          {task.resultHash && <div className={styles.hashRow}>Result Hash: <a className={styles.hashLink} href="#">{task.resultHash}</a></div>}
        </div>
      )}
      {task.evaluation && <EvaluationPanel evaluation={task.evaluation} />}
      <TransactionsList hashes={task.transactionHashes} />
    </div>
  );
}

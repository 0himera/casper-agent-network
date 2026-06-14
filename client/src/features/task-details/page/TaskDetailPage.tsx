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
import { SkeletonDetail } from "@/shared/ui";
import { motion } from "motion/react";
import styles from "@/features/task-details/ui/TaskDetail.module.css";

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

export default function TaskDetailPage({ params }: { params: Promise<{ taskId: string }> }) {
  const { taskId } = use(params);
  const { data: task, isLoading } = useTaskByIdQuery(taskId);

  if (isLoading) {
    return (
      <div className={styles.page}>
        <SkeletonDetail />
      </div>
    );
  }
  if (!task) return <div className={styles.loading}>Task not found</div>;

  const steps = [
    { label: "Created", time: formatTimeAgo(task.createdAt) },
    { label: "Assigned", time: task.assignedAgentName ? formatTimeAgo(task.createdAt) : null, detail: task.assignedAgentName ?? undefined },
    { label: "In Progress", time: task.status !== "open" ? formatTimeAgo(task.updatedAt) : null },
    { label: "Submitted", time: task.result ? formatTimeAgo(task.updatedAt) : null },
    { label: "Completed", time: task.status === "completed" ? formatTimeAgo(task.updatedAt) : null, detail: task.status === "completed" ? "Escrow released" : undefined },
  ];

  return (
    <motion.div
      className={styles.page}
      variants={containerVariants}
      initial="hidden"
      animate="show"
    >
      <motion.div variants={itemVariants}>
        <Link href="/tasks" className={styles.backLink}><ArrowLeft size={16} /> Back to Job Board</Link>
      </motion.div>
      <motion.div variants={itemVariants} className={styles.header}>
        <div className={styles.titleRow}><h1 className={styles.title}>{task.prompt.slice(0, 60)}...</h1></div>
        <div className={styles.meta}>
          <span className={styles.metaItem}><ClipboardList size={13} /> {SKILL_LABELS[task.domain]}</span>
          <span className={styles.metaItem}><Coins size={13} /> {task.budget} CSPR</span>
          <span className={styles.metaItem}><Hash size={13} /> {task.id}</span>
        </div>
      </motion.div>
      <motion.div variants={itemVariants}>
        <StatusTimeline steps={steps} />
      </motion.div>
      {task.result && (
        <motion.div variants={itemVariants} className={styles.section}>
          <h3 className={styles.sectionTitle}>Result</h3>
          <div className={styles.resultContent}>{task.result}</div>
          {task.resultHash && <div className={styles.hashRow}>Result Hash: <a className={styles.hashLink} href="#">{task.resultHash}</a></div>}
        </motion.div>
      )}
      {task.evaluation && (
        <motion.div variants={itemVariants}>
          <EvaluationPanel evaluation={task.evaluation} />
        </motion.div>
      )}
      <motion.div variants={itemVariants}>
        <TransactionsList hashes={task.transactionHashes} />
      </motion.div>
    </motion.div>
  );
}

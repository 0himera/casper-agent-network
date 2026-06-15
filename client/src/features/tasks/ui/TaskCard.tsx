import Link from "next/link";
import { Coins, Clock, User, Bot } from "lucide-react";
import type { TaskEntity } from "@/entities/task/types/types";
import { TaskStatusBadge } from "@/entities/task/ui/TaskStatusBadge";
import { truncateAddress, formatDeadline } from "@/shared/utils/format";
import { motion } from "motion/react";
import styles from "./Tasks.module.css";

interface TaskCardProps { task: TaskEntity }

const itemVariants = {
  hidden: { opacity: 0, y: 12, scale: 0.98 },
  show: {
    opacity: 1,
    y: 0,
    scale: 1,
    transition: {
      type: "spring",
      stiffness: 100,
      damping: 15,
    },
  },
} as const;

const MotionLink = motion.create(Link);

export function TaskCard({ task }: TaskCardProps) {
  const deadline = formatDeadline(task.deadline);
  const isExpired = deadline === "Expired";

  return (
    <MotionLink
      href={`/tasks/${task.id}`}
      className={styles.taskCard}
      variants={itemVariants}
      whileHover={{
        y: -3,
        scale: 1.01,
        borderColor: "rgba(143, 174, 139, 0.4)",
        boxShadow: "0 10px 25px rgba(0, 0, 0, 0.15)",
      }}
      transition={{ duration: 0.2 }}
    >
      <div className={styles.taskCardHeader}>
        <span className={styles.taskId}>{task.id}</span>
        <TaskStatusBadge status={task.status} />
      </div>
      <div className={styles.taskPrompt}>{task.prompt}</div>
      <div className={styles.taskMeta}>
        <span className={styles.taskMetaItem}>
          <Coins className={styles.taskMetaIcon} />{task.budget} CSPR
        </span>
        <span className={styles.taskMetaItem}>
          <User className={styles.taskMetaIcon} />{truncateAddress(task.creator)}
        </span>
        {task.assignedAgentName && (
          <span className={styles.taskMetaItem}>
            <Bot className={styles.taskMetaIcon} />{task.assignedAgentName}
          </span>
        )}
        <span className={`${styles.taskMetaItem} ${isExpired ? styles.deadlineExpired : ""}`}>
          <Clock className={styles.taskMetaIcon} />{deadline}
        </span>
      </div>
    </MotionLink>
  );
}

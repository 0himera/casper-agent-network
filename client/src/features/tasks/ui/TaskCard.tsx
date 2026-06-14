import Link from "next/link";
import { Coins, Clock, User, Bot } from "lucide-react";
import type { TaskEntity } from "@/entities/task/types/types";
import { TaskStatusBadge } from "@/entities/task/ui/TaskStatusBadge";
import { truncateAddress, formatDeadline } from "@/shared/utils/format";
import styles from "./Tasks.module.css";

interface TaskCardProps { task: TaskEntity }

export function TaskCard({ task }: TaskCardProps) {
  const deadline = formatDeadline(task.deadline);
  const isExpired = deadline === "Expired";

  return (
    <Link href={`/tasks/${task.id}`} className={styles.taskCard}>
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
    </Link>
  );
}

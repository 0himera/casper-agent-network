import type { TaskStatus } from "@/entities/task/types/types";
import styles from "./TaskStatusBadge.module.css";

const STATUS_MAP: Record<TaskStatus, string> = {
  open: styles.open,
  in_progress: styles.inProgress,
  completed: styles.completed,
  cancelled: styles.cancelled,
};

const LABELS: Record<TaskStatus, string> = {
  open: "Open",
  in_progress: "In Progress",
  completed: "Completed",
  cancelled: "Cancelled",
};

interface TaskStatusBadgeProps {
  status: TaskStatus;
}

export function TaskStatusBadge({ status }: TaskStatusBadgeProps) {
  return (
    <span className={`${styles.badge} ${STATUS_MAP[status]}`}>
      {LABELS[status]}
    </span>
  );
}

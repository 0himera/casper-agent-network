"use client";

import { CheckCircle, XCircle, Info, X } from "lucide-react";
import { motion, AnimatePresence } from "motion/react";
import { useToastStore, toast as toastApi } from "@/shared/store/toast-store";
import styles from "./Toast.module.css";

export const toast = toastApi;

const icons = {
  success: CheckCircle,
  error: XCircle,
  info: Info,
};

export function ToastContainer() {
  const toasts = useToastStore((s) => s.toasts);

  return (
    <div className={styles.container} aria-live="polite" aria-atomic="true" role="status">
      <AnimatePresence mode="popLayout">
        {toasts.map((t) => (
          <ToastItem key={t.id} toast={t} />
        ))}
      </AnimatePresence>
    </div>
  );
}

function ToastItem({
  toast,
}: {
  toast: { id: string; type: "success" | "error" | "info"; message: string };
}) {
  const dismiss = useToastStore((s) => s.dismiss);
  const Icon = icons[toast.type];

  return (
    <motion.div
      layout
      initial={{ opacity: 0, x: 40, scale: 0.95 }}
      animate={{ opacity: 1, x: 0, scale: 1 }}
      exit={{ opacity: 0, x: 20, scale: 0.95 }}
      transition={{ type: "spring", stiffness: 300, damping: 24 }}
      className={`${styles.toast} ${styles[toast.type]}`}
    >
      <Icon size={18} aria-hidden="true" className={styles.icon} />
      <span className={styles.message}>{toast.message}</span>
      <button
        type="button"
        onClick={() => dismiss(toast.id)}
        className={styles.close}
        aria-label="Dismiss notification"
      >
        <X size={14} aria-hidden="true" />
      </button>
    </motion.div>
  );
}

"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { SKILL_BASE_PRICES } from "@/entities/agent/types/types";
import type { AgentSkill } from "@/entities/agent/types/types";
import { DomainField } from "@/features/tasks/ui/DomainField";
import { BudgetField } from "@/features/tasks/ui/BudgetField";
import { motion } from "motion/react";
import { buildCreateTaskTx } from "@/shared/utils/contract-transactions";
import { signAndSendTransaction } from "@/features/wallet/utils/signing";
import { apiPost } from "@/shared/api/api-client";
import { useAppStore } from "@/shared/providers/AppStoreProvider";
import { toast } from "@/shared/ui/Toast";
import styles from "@/features/tasks/ui/CreateTask.module.css";

export default function CreateTaskPage() {
  const router = useRouter();
  const walletAddress = useAppStore((s) => s.walletAddress);

  const [domain, setDomain] = useState<AgentSkill>("defi_analysis");
  const [budget, setBudget] = useState("5.0");
  const [prompt, setPrompt] = useState("");
  const [deadline, setDeadline] = useState("");
  const [parentTaskId, setParentTaskId] = useState("");
  const [loading, setLoading] = useState(false);
  const [status, setStatus] = useState("");

  const [taskId] = useState(() => `task_${Math.random().toString(36).slice(2, 10)}`);
  const isValid = Number(budget) >= 1 && prompt.trim().length > 0;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!walletAddress) {
      toast.error("Please connect your Casper Wallet first.");
      return;
    }

    setLoading(true);
    setStatus("Building transaction...");
    try {
      const budgetMotes = String(Math.round(parseFloat(budget) * 1_000_000_000));
      const deadlineMs = deadline ? new Date(deadline).getTime() : Date.now() + 24 * 60 * 60 * 1000;

      const transaction = await buildCreateTaskTx(
        walletAddress,
        taskId,
        budgetMotes,
        `https://agentnetwork.io/task/${taskId}`,
        deadlineMs,
        parentTaskId.trim() || undefined,
      );
      setStatus("Signing transaction...");
      const txHash = await signAndSendTransaction(transaction, walletAddress);

      setStatus("Saving task configuration to backend...");
      await apiPost("/api/tasks", {
        id: taskId,
        creator_public_key: walletAddress,
        budget_motes: parseInt(budgetMotes, 10),
        transaction_hash: txHash,
        domain: domain,
        prompt: prompt,
        deadline: deadlineMs,
        parent_task_id: parentTaskId.trim() || null,
      });

      setStatus("Task successfully created!");
      toast.success(`Task created on-chain and off-chain!\nTransaction Hash: ${txHash}`);
      router.push("/tasks");
    } catch (err: unknown) {
      console.error(err);
      setStatus("");
      toast.error(`Failed to create task: ${String(err)}`);
    } finally {
      setLoading(false);
    }
  };

  return (
    <motion.div
      className={styles.page}
      initial={{ opacity: 0, y: 15 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.35, ease: "easeOut" }}
    >
      <h1 className={styles.title}>Create New Task</h1>
      <form className={styles.form} onSubmit={handleSubmit}>
        <div className={styles.field}>
          <label className={styles.label}>Task ID</label>
          <input className={styles.readonlyInput} value={taskId} readOnly />
        </div>
        <DomainField value={domain} onChange={setDomain} />
        <BudgetField value={budget} onChange={setBudget} recommended={SKILL_BASE_PRICES[domain]} />
        <div className={styles.field}>
          <label className={styles.label}>Parent Task ID (Optional - For A2A Swarms)</label>
          <input
            className={styles.input}
            value={parentTaskId}
            onChange={(e) => setParentTaskId(e.target.value)}
            placeholder="e.g. task_abcdef"
            disabled={loading}
          />
        </div>
        <div className={styles.field}>
          <label className={styles.label}>Prompt</label>
          <textarea
            className={styles.textarea}
            value={prompt}
            onChange={(e) => setPrompt(e.target.value)}
            placeholder="Describe the task..."
            disabled={loading}
          />
        </div>
        <div className={styles.field}>
          <label className={styles.label}>Deadline</label>
          <input
            type="datetime-local"
            className={styles.input}
            value={deadline}
            onChange={(e) => setDeadline(e.target.value)}
            disabled={loading}
          />
          <span className={styles.hint}>Default: +24 hours from now</span>
        </div>

        {status && (
          <div
            className={styles.statusMessage}
            aria-live="polite"
            aria-atomic="true"
            style={{
              padding: "12px",
              background: "rgba(255,255,255,0.05)",
              borderRadius: "8px",
              border: "1px solid rgba(255,255,255,0.1)",
              fontSize: "0.9rem",
              color: "var(--text-muted)",
              marginBottom: "16px",
            }}
          >
            {status}
          </div>
        )}

        <div className={styles.actions}>
          <button
            type="button"
            className={styles.cancelButton}
            onClick={() => router.back()}
            disabled={loading}
          >
            Cancel
          </button>
          <motion.button
            whileHover={{ scale: isValid && !loading ? 1.01 : 1 }}
            whileTap={{ scale: isValid && !loading ? 0.99 : 1 }}
            type="submit"
            className={styles.submitButton}
            disabled={!isValid || loading}
          >
            {loading ? "Processing..." : "Create Task & Lock Escrow"}
          </motion.button>
        </div>
      </form>
    </motion.div>
  );
}

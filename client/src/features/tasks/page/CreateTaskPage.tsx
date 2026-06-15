"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { SKILL_BASE_PRICES } from "@/entities/agent/types/types";
import type { AgentSkill } from "@/entities/agent/types/types";
import { DomainField } from "@/features/tasks/ui/DomainField";
import { BudgetField } from "@/features/tasks/ui/BudgetField";
import { motion } from "motion/react";
import styles from "@/features/tasks/ui/CreateTask.module.css";

export default function CreateTaskPage() {
  const router = useRouter();
  const [domain, setDomain] = useState<AgentSkill>("defi_analysis");
  const [budget, setBudget] = useState("5.0");
  const [prompt, setPrompt] = useState("");
  const [deadline, setDeadline] = useState("");

  const taskId = `task_${Math.random().toString(36).slice(2, 10)}`;
  const isValid = Number(budget) >= 1 && prompt.trim().length > 0;

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    alert(`Task ${taskId} created! Budget: ${budget} CSPR`);
    router.push("/tasks");
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
          <label className={styles.label}>Prompt</label>
          <textarea className={styles.textarea} value={prompt} onChange={(e) => setPrompt(e.target.value)} placeholder="Describe the task..." />
        </div>
        <div className={styles.field}>
          <label className={styles.label}>Deadline</label>
          <input type="datetime-local" className={styles.input} value={deadline} onChange={(e) => setDeadline(e.target.value)} />
          <span className={styles.hint}>Default: +24 hours from now</span>
        </div>
        <div className={styles.actions}>
          <button type="button" className={styles.cancelButton} onClick={() => router.back()}>Cancel</button>
          <motion.button
            whileHover={{ scale: isValid ? 1.01 : 1 }}
            whileTap={{ scale: isValid ? 0.99 : 1 }}
            type="submit"
            className={styles.submitButton}
            disabled={!isValid}
          >
            Create Task &amp; Lock {budget} CSPR
          </motion.button>
        </div>
      </form>
    </motion.div>
  );
}

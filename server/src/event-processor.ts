/**
 * Testable event-processing seam for Wave 4 scenarios E17–E19.
 * Extracted from event-handler so synthetic events can run without WebSocket.
 */
import { Pool, RowDataPacket } from "mysql2/promise";

export type EventProcessorDeps = {
  pool: Pool;
  /** Optional: resolve account-hash → public_key. Defaults to identity. */
  resolveAccount?: (raw: string) => Promise<string>;
  /** Optional: trigger backend /execute|/validate. Defaults to no-op. */
  triggerBackend?: (
    taskId: string,
    action: "execute" | "validate"
  ) => Promise<void>;
  /** Optional: backend health. Defaults to false (skip auto validate/execute). */
  checkBackendHealth?: () => Promise<boolean>;
  log?: (msg: string) => void;
  error?: (msg: string) => void;
};

export type SyntheticContractEvent = {
  timestamp?: string;
  extra?: { deploy_hash?: string };
  data: {
    name: string;
    data: Record<string, unknown>;
  };
};

export type ProcessResult =
  | { ok: true; action: string }
  | { ok: false; reason: string };

/**
 * Security validation matching event-handler.ts rules.
 * Returns null if valid, or a rejection reason string.
 */
export function validateEventSecurity(
  payload: Record<string, unknown> | null | undefined
): string | null {
  if (!payload || typeof payload !== "object") {
    return null;
  }

  if ("task_id" in payload && payload.task_id !== undefined && payload.task_id !== null) {
    const taskId = String(payload.task_id);
    if (!/^[a-zA-Z0-9_-]+$/.test(taskId)) {
      return `invalid task_id format: "${taskId}"`;
    }
  }
  if (
    "parent_task_id" in payload &&
    payload.parent_task_id !== undefined &&
    payload.parent_task_id !== null
  ) {
    const parentTaskId = String(payload.parent_task_id);
    if (!/^[a-zA-Z0-9_-]+$/.test(parentTaskId)) {
      return `invalid parent_task_id format: "${parentTaskId}"`;
    }
  }

  const actorFields = ["agent", "creator", "validator", "disputer"];
  for (const field of actorFields) {
    if (field in payload && payload[field] !== undefined && payload[field] !== null) {
      const actorVal = String(payload[field]);
      if (!/^(account-hash-)?[a-fA-F0-9]{64,66}$/.test(actorVal)) {
        return `invalid ${field} format: "${actorVal}"`;
      }
    }
  }

  return null;
}

/**
 * Process a single contract event against the DB (idempotent upserts where applicable).
 */
export async function processContractEvent(
  event: SyntheticContractEvent,
  deps: EventProcessorDeps
): Promise<ProcessResult> {
  const log = deps.log ?? console.log;
  const error = deps.error ?? console.error;
  const pool = deps.pool;
  const resolveAccount =
    deps.resolveAccount ?? (async (raw: string) => raw);
  const checkBackendHealth =
    deps.checkBackendHealth ?? (async () => false);
  const triggerBackend =
    deps.triggerBackend ?? (async () => undefined);

  const eventName = event.data?.name;
  const payload = (event.data?.data ?? {}) as Record<string, unknown>;

  const securityFail = validateEventSecurity(payload);
  if (securityFail) {
    error(`Security Warning: Rejected event due to ${securityFail}`);
    return { ok: false, reason: securityFail };
  }

  try {
    if (eventName === "TaskSubmitted") {
      const taskId = String(payload.task_id ?? "");
      const resultHash = String(payload.result_hash ?? "");
      if (!taskId || !resultHash) {
        return { ok: false, reason: "missing task_id or result_hash" };
      }

      await pool.execute(
        "UPDATE tasks SET result_hash = ? WHERE id = ?",
        [resultHash, taskId]
      );
      log(`Result submitted for task ${taskId}: ${resultHash}`);

      const healthy = await checkBackendHealth();
      if (healthy) {
        await triggerBackend(taskId, "validate").catch((err: Error) => {
          log(`Error triggering validation on backend: ${err.message || err}`);
        });
      } else {
        log(`Backend is unhealthy. Skipping automated validation for task ${taskId}.`);
      }
      return { ok: true, action: "TaskSubmitted" };
    }

    if (eventName === "TaskCompleted") {
      const taskId = String(payload.task_id ?? "");
      if (!taskId) {
        return { ok: false, reason: "missing task_id" };
      }

      const [taskRows] = await pool.query<RowDataPacket[]>(
        "SELECT * FROM tasks WHERE id = ?",
        [taskId]
      );
      const task = taskRows[0];

      if (task) {
        await pool.execute(
          'UPDATE tasks SET status = "Completed" WHERE id = ?',
          [taskId]
        );
        if (task.assigned_agent_public_key) {
          await pool.execute(
            "UPDATE agents SET active_jobs = GREATEST(0, active_jobs - 1) WHERE public_key = ?",
            [task.assigned_agent_public_key]
          );
        }
      }
      log(`Task ${taskId} marked as completed`);
      return { ok: true, action: "TaskCompleted" };
    }

    if (eventName === "TaskCreated") {
      const taskId = String(payload.task_id ?? "");
      const creatorRaw = String(payload.creator ?? "");
      if (!taskId || !creatorRaw) {
        return { ok: false, reason: "missing task_id or creator" };
      }
      const creator = await resolveAccount(creatorRaw);
      const budget = Number(payload.budget ?? 0);
      const domain = String(payload.domain ?? "defi_analysis");
      const prompt = String(payload.prompt ?? "");
      const deadline = Number(payload.deadline ?? 0);
      const deployHash = event.extra?.deploy_hash ?? "synthetic-deploy";
      const parentTaskId =
        payload.parent_task_id === undefined || payload.parent_task_id === null
          ? null
          : String(payload.parent_task_id);

      const [existing] = await pool.query<RowDataPacket[]>(
        "SELECT id FROM tasks WHERE id = ?",
        [taskId]
      );
      if (existing.length === 0) {
        // Mirror event-handler.ts: persist parent_task_id on insert.
        await pool.execute(
          `INSERT INTO tasks (
            id, creator_public_key, assigned_agent_public_key, budget_motes, status,
            transaction_hash, domain, prompt, deadline, timestamp, parent_task_id
          ) VALUES (?, ?, NULL, ?, 'Open', ?, ?, ?, ?, NOW(), ?)`,
          [taskId, creator, budget, deployHash, domain, prompt, deadline, parentTaskId]
        );
      } else {
        // Mirror production IFNULL semantics: first non-null parent wins.
        await pool.execute(
          "UPDATE tasks SET parent_task_id = IFNULL(parent_task_id, ?) WHERE id = ?",
          [parentTaskId, taskId]
        );
      }
      log(`Task created/ensured: ${taskId}`);
      return { ok: true, action: "TaskCreated" };
    }

    if (eventName === "ValidationSubmitted") {
      log("Validation Submitted by validator for a task");
      return { ok: true, action: "ValidationSubmitted" };
    }

    return { ok: false, reason: `unsupported event: ${eventName}` };
  } catch (err: any) {
    error(`Error processing event: ${err?.message || err}`);
    return { ok: false, reason: `db_or_runtime: ${err?.message || String(err)}` };
  }
}

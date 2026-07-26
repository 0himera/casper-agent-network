/**
 * Startup catch-up for missed CSPR.cloud stream events.
 * Uses recent package deploys (proxy entry_point + nested runtime args) to
 * backfill TaskCreated / TaskAssigned into MySQL after reconnect.
 */
import axios from "axios";
import { Pool, RowDataPacket } from "mysql2/promise";

type CatchUpDeps = {
  pool: Pool;
  apiUrl: string;
  accessKey: string;
  contractPackageHash: string;
  log?: (msg: string) => void;
  /** How many newest deploys to scan (page 1). */
  limit?: number;
};

function packageHashBare(hash: string): string {
  return hash.replace(/^hash-/, "");
}

/** Extract printable task_id values from proxy RuntimeArgs byte list. */
export function extractTaskIdFromArgBytes(bytes: number[]): string | null {
  const text = Buffer.from(bytes).toString("latin1");
  // Prefer values after the `task_id` name so we do not match the name itself.
  const afterName = text.split("task_id")[1] || text;
  const match =
    afterName.match(/task_w5_[a-zA-Z0-9_]+/) ||
    afterName.match(/task_[a-zA-Z0-9]+_[0-9]+/) ||
    afterName.match(/task_[a-zA-Z0-9_-]{8,}/);
  return match ? match[0] : null;
}

/**
 * Extract account-hash (64 hex chars) that follows the `agent` named arg
 * in assign_task proxy RuntimeArgs.
 *
 * Observed encoding after name `agent`:
 *   u32 le length=33, tag=0 (Key::Account), then 32-byte account hash.
 */
export function extractAgentHashFromArgBytes(bytes: number[]): string | null {
  const buf = Buffer.from(bytes);
  const text = buf.toString("latin1");
  const agentPos = text.indexOf("agent");
  if (agentPos < 0) return null;

  for (let i = agentPos + 5; i < buf.length - 37; i++) {
    // u32 little-endian 33, then Account key tag 0, then 32-byte hash
    if (
      buf[i] === 33 &&
      buf[i + 1] === 0 &&
      buf[i + 2] === 0 &&
      buf[i + 3] === 0 &&
      buf[i + 4] === 0
    ) {
      return buf.subarray(i + 5, i + 37).toString("hex");
    }
  }
  return null;
}

function nestedArgBytes(deploy: any): number[] | null {
  const parsed = deploy?.args?.args?.parsed;
  if (!Array.isArray(parsed) || parsed.length === 0) return null;
  if (typeof parsed[0] === "number") return parsed as number[];
  return null;
}

function entryPointName(deploy: any): string | null {
  const ep = deploy?.args?.entry_point?.parsed;
  return typeof ep === "string" ? ep : null;
}

async function ensureTaskCreated(
  pool: Pool,
  taskId: string,
  creator: string,
  deployHash: string,
  log: (m: string) => void
): Promise<void> {
  const [rows] = await pool.query<RowDataPacket[]>(
    "SELECT id, status FROM tasks WHERE id = ?",
    [taskId]
  );
  if (rows[0]) {
    log(`catch-up: TaskCreated skip (already present) ${taskId} status=${rows[0].status}`);
    return;
  }
  await pool.execute(
    "INSERT INTO tasks (id, creator_public_key, budget_motes, status, transaction_hash, domain, prompt, deadline, timestamp) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    [
      taskId,
      creator,
      0,
      "Open",
      deployHash,
      "defi_analysis",
      "",
      "0",
      new Date(),
    ]
  );
  log(`catch-up: TaskCreated inserted ${taskId}`);
}

async function ensureTaskAssigned(
  pool: Pool,
  taskId: string,
  agentHash: string,
  deployHash: string,
  log: (m: string) => void
): Promise<void> {
  const agentKey = agentHash.startsWith("account-hash-")
    ? agentHash
    : `account-hash-${agentHash}`;
  const cleanHash = agentKey.replace(/^account-hash-/, "");

  await pool.execute(
    "INSERT IGNORE INTO agents (public_key, name, description, active_jobs, status, timestamp) VALUES (?, ?, ?, ?, ?, ?)",
    [
      agentKey,
      `Agent ${cleanHash.slice(0, 8)}`,
      "Registered via catch-up assign",
      0,
      "active",
      new Date(),
    ]
  );

  const [rows] = await pool.query<RowDataPacket[]>(
    "SELECT id, status, assigned_agent_public_key FROM tasks WHERE id = ?",
    [taskId]
  );
  if (!rows[0]) {
    await pool.execute(
      "INSERT INTO tasks (id, creator_public_key, budget_motes, status, transaction_hash, domain, prompt, deadline, timestamp, assigned_agent_public_key) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
      [
        taskId,
        "pending_creator",
        0,
        "InProgress",
        deployHash,
        "defi_analysis",
        "",
        "0",
        new Date(),
        agentKey,
      ]
    );
    log(`catch-up: TaskAssigned inserted stub ${taskId} -> ${agentKey}`);
    return;
  }
  if (rows[0].status === "InProgress" && rows[0].assigned_agent_public_key) {
    log(`catch-up: TaskAssigned skip (already InProgress) ${taskId}`);
    return;
  }
  await pool.execute(
    'UPDATE tasks SET assigned_agent_public_key = ?, status = "InProgress", transaction_hash = COALESCE(transaction_hash, ?) WHERE id = ?',
    [agentKey, deployHash, taskId]
  );
  log(`catch-up: TaskAssigned updated ${taskId} -> ${agentKey}`);
}

/**
 * Scan recent contract-package deploys and backfill create/assign transitions.
 */
export async function catchUpFromRecentDeploys(deps: CatchUpDeps): Promise<number> {
  const log = deps.log ?? console.log;
  const limit = deps.limit ?? 40;
  const pkg = packageHashBare(deps.contractPackageHash);
  const url = `${deps.apiUrl.replace(/\/$/, "")}/deploys`;

  let data: any;
  try {
    const resp = await axios.get(url, {
      headers: { authorization: deps.accessKey },
      params: { contract_package_hash: pkg, page: 1, limit },
      timeout: 15000,
    });
    data = resp.data;
  } catch (err: any) {
    log(`catch-up: failed to fetch recent deploys: ${err.message || err}`);
    return 0;
  }

  const items: any[] = Array.isArray(data?.data) ? data.data : [];
  // Process oldest → newest within the page so create precedes assign.
  const ordered = [...items].reverse();
  let applied = 0;

  for (const deploy of ordered) {
    const status = String(deploy?.status || "");
    if (!["processed", "executed", "success"].includes(status)) continue;
    if (deploy?.error_message) continue;

    const ep = entryPointName(deploy);
    if (ep !== "create_task" && ep !== "assign_task") continue;

    const bytes = nestedArgBytes(deploy);
    if (!bytes) continue;
    const taskId = extractTaskIdFromArgBytes(bytes);
    if (!taskId) continue;

    const deployHash = String(deploy.deploy_hash || "");
    try {
      if (ep === "create_task") {
        const creator =
          deploy.caller_public_key ||
          (deploy.caller_hash
            ? `account-hash-${String(deploy.caller_hash).replace(/^account-hash-/, "")}`
            : "unknown_creator");
        await ensureTaskCreated(deps.pool, taskId, creator, deployHash, log);
        applied += 1;
      } else if (ep === "assign_task") {
        const agentHash = extractAgentHashFromArgBytes(bytes);
        if (!agentHash) {
          log(`catch-up: assign_task ${taskId} missing agent hash`);
          continue;
        }
        await ensureTaskAssigned(deps.pool, taskId, agentHash, deployHash, log);
        applied += 1;
      }
    } catch (err: any) {
      log(`catch-up: error applying ${ep} for ${taskId}: ${err.message || err}`);
    }
  }

  log(`catch-up: scanned ${items.length} deploys, applied ${applied} create/assign updates`);
  return applied;
}

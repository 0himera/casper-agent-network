/**
 * Wave 4 E17–E19: synthetic event-handler tests (no WebSocket, no CSPR.cloud).
 */
import { pool } from "./db";
import {
  processContractEvent,
  validateEventSecurity,
  SyntheticContractEvent,
} from "./event-processor";
import * as assert from "assert";
import { execSync } from "child_process";

const AGENT_PK = "a".repeat(64);
const CREATOR_PK = "b".repeat(64);
const TASK_ID = "w4-eh-task-1";

function makeEvent(
  name: string,
  data: Record<string, unknown>
): SyntheticContractEvent {
  return {
    timestamp: new Date().toISOString(),
    extra: { deploy_hash: "w4-deploy-hash" },
    data: { name, data },
  };
}

async function cleanup() {
  await pool.query("DELETE FROM validations WHERE task_id = ?", [TASK_ID]);
  await pool.query("DELETE FROM tasks WHERE id = ?", [TASK_ID]);
  await pool.query("DELETE FROM agents WHERE public_key IN (?, ?)", [
    AGENT_PK,
    CREATOR_PK,
  ]);
}

async function seedBase() {
  await pool.query(
    `INSERT INTO agents (public_key, name, status, active_jobs, is_available)
     VALUES (?, 'W4 EH Agent', 'active', 1, 1)
     ON DUPLICATE KEY UPDATE status='active', active_jobs=1`,
    [AGENT_PK]
  );
  await pool.query(
    `INSERT INTO tasks (
      id, creator_public_key, assigned_agent_public_key, budget_motes, status,
      transaction_hash, domain, prompt, deadline, timestamp
     ) VALUES (?, ?, ?, 100, 'InProgress', 'w4-tx', 'defi_analysis', 'prompt', 999999, NOW())
     ON DUPLICATE KEY UPDATE status='InProgress', result_hash=NULL, result=NULL`,
    [TASK_ID, CREATOR_PK, AGENT_PK]
  );
}

async function runTests() {
  console.log("=== Running Event-Handler Synthetic Tests (Wave 4 E17–E19) ===");

  try {
    // Unit: security validation (E18)
    {
      const bad = validateEventSecurity({
        task_id: "../etc/passwd",
        creator: CREATOR_PK,
      });
      assert.ok(bad && bad.includes("task_id"), "E18: reject path-like task_id");
      console.log("  [PASS] scenario 18a: invalid task_id rejected by security gate");

      const badActor = validateEventSecurity({
        task_id: TASK_ID,
        validator: "not-a-hex",
      });
      assert.ok(
        badActor && badActor.includes("validator"),
        "E18: reject bad validator"
      );
      console.log("  [PASS] scenario 18b: empty/invalid validator address rejected");
    }

    await cleanup();
    await seedBase();

    const deps = {
      pool,
      checkBackendHealth: async () => false,
      triggerBackend: async () => undefined,
    };

    // E17: out-of-order TaskCompleted before TaskSubmitted, then replay both
    {
      const completed = makeEvent("TaskCompleted", {
        task_id: TASK_ID,
        // creator omitted — TaskCompleted payload in prod may vary; security allows missing
      });
      const r1 = await processContractEvent(completed, deps);
      assert.strictEqual(r1.ok, true, "E17: TaskCompleted ok even before submit");

      const [rows1] = await pool.query<any[]>(
        "SELECT status, result_hash FROM tasks WHERE id = ?",
        [TASK_ID]
      );
      assert.strictEqual(rows1[0].status, "Completed");
      assert.strictEqual(rows1[0].result_hash, null);
      console.log("  [PASS] scenario 17a: TaskCompleted before TaskSubmitted does not crash");

      const submitted = makeEvent("TaskSubmitted", {
        task_id: TASK_ID,
        result_hash: "hash-abc",
      });
      const r2 = await processContractEvent(submitted, deps);
      assert.strictEqual(r2.ok, true);

      // Replay both
      await processContractEvent(completed, deps);
      await processContractEvent(submitted, deps);

      const [rows2] = await pool.query<any[]>(
        "SELECT status, result_hash FROM tasks WHERE id = ?",
        [TASK_ID]
      );
      assert.strictEqual(rows2[0].status, "Completed", "E17: status not regressed");
      assert.strictEqual(rows2[0].result_hash, "hash-abc");
      console.log("  [PASS] scenario 17: out-of-order + replay is idempotent");
    }

    // E18: partially broken payload via processContractEvent
    {
      const bad = await processContractEvent(
        makeEvent("TaskSubmitted", {
          task_id: "bad id!!",
          result_hash: "x",
        }),
        deps
      );
      assert.strictEqual(bad.ok, false);
      assert.ok(String(bad.reason).includes("task_id"));
      console.log("  [PASS] scenario 18: broken payload rejected with clear reason");
    }

    // G25: TaskCreated persists parent_task_id (aligned with event-handler.ts)
    {
      const childId = "w4-eh-child-1";
      const parentId = "w4-eh-parent-1";
      await pool.query("DELETE FROM tasks WHERE id = ?", [childId]);
      const created = await processContractEvent(
        makeEvent("TaskCreated", {
          task_id: childId,
          creator: CREATOR_PK,
          budget: 1000,
          deadline: 999999,
          parent_task_id: parentId,
        }),
        deps
      );
      assert.strictEqual(created.ok, true, "G25 TaskCreated ok");
      const [rows] = await pool.query<any[]>(
        "SELECT parent_task_id FROM tasks WHERE id = ?",
        [childId]
      );
      assert.strictEqual(
        rows[0]?.parent_task_id,
        parentId,
        "G25: event-processor must persist parent_task_id"
      );
      await pool.query("DELETE FROM tasks WHERE id = ?", [childId]);
      console.log("  [PASS] G25: TaskCreated persists parent_task_id");
    }

    // E19: DB failure during upsert — prefer docker stop; fall back to dead pool
    // when docker.sock is unavailable (e.g. restricted CI sandboxes).
    {
      let sawFailure = false;
      let usedDocker = false;
      try {
        try {
          execSync("docker stop agent-network-mysql", { stdio: "pipe" });
          usedDocker = true;
          await new Promise((r) => setTimeout(r, 500));
          const r = await processContractEvent(
            makeEvent("TaskSubmitted", {
              task_id: TASK_ID,
              result_hash: "hash-after-kill",
            }),
            {
              ...deps,
              error: () => {
                /* silence */
              },
            }
          );
          assert.strictEqual(r.ok, false, "E19: expect failure when DB down");
          assert.ok(
            String(r.reason).startsWith("db_or_runtime:"),
            `E19: reason=${r.reason}`
          );
          sawFailure = true;
          console.log(
            "  [PASS] scenario 19: DB failure returns controlled error, no throw"
          );
        } catch (dockerErr: any) {
          const msg = String(dockerErr?.stderr || dockerErr?.message || dockerErr);
          if (!msg.includes("permission denied") && !msg.includes("Cannot connect")) {
            throw dockerErr;
          }
          // Dead-pool fallback (mirrors Rust wave4 lazy-pool pattern).
          const mysql = await import("mysql2/promise");
          const deadPool = mysql.createPool({
            host: "127.0.0.1",
            port: 1,
            user: "deagentnet",
            password: "passw0rd",
            database: "deagentnet",
            connectTimeout: 1000,
          });
          try {
            const r = await processContractEvent(
              makeEvent("TaskSubmitted", {
                task_id: TASK_ID,
                result_hash: "hash-after-kill",
              }),
              {
                pool: deadPool,
                checkBackendHealth: async () => false,
                triggerBackend: async () => undefined,
                error: () => {
                  /* silence */
                },
              }
            );
            assert.strictEqual(r.ok, false, "E19: expect failure with dead pool");
            assert.ok(
              String(r.reason).startsWith("db_or_runtime:"),
              `E19: reason=${r.reason}`
            );
            sawFailure = true;
            console.log(
              "  [PASS] scenario 19: DB failure via dead pool (docker unavailable)"
            );
          } finally {
            await deadPool.end().catch(() => undefined);
          }
        }
      } finally {
        if (usedDocker) {
          try {
            execSync("docker start agent-network-mysql", { stdio: "pipe" });
            for (let i = 0; i < 30; i++) {
              try {
                await pool.query("SELECT 1");
                break;
              } catch {
                await new Promise((r) => setTimeout(r, 500));
              }
            }
          } catch (e) {
            console.error("Failed to restart MySQL:", e);
          }
        }
      }
      assert.ok(sawFailure, "E19 must observe failure");
    }

    console.log("\n=== ALL EVENT-HANDLER TESTS PASSED ===");
  } finally {
    try {
      await cleanup();
    } catch {
      /* DB may still be recovering */
    }
    await pool.end().catch(() => undefined);
  }
}

runTests().catch((e) => {
  console.error("Test execution failed:", e);
  process.exit(1);
});

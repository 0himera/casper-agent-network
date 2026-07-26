/**
 * Wave 4 G25/G26: MCP write-tool semantic parity + parent_task_id read surface.
 * No testnet — asserts unsigned tx shape / decoded fields locally.
 */
import { config } from "./config";
import { server } from "./mcp-server";
import { pool } from "./db";
import * as assert from "assert";

// Ensure package hash for SessionBuilder (loaded at import; may be unset in CI).
if (!config.contractPackageHash) {
  config.contractPackageHash =
    process.env.CONTRACT_PACKAGE_HASH ||
    "e8e0cba1a3e6c8d2f17a51066d60ebaae764e54e5476ebb965eadff6e56dc699";
}

const SENDER = "01" + "ab".repeat(32);
const AGENT = "01" + "cd".repeat(32);
const ADMIN = "w4-parity-admin-token";
const PARENT_ID = "w4-parity-parent_1";
const CHILD_ID = "w4-parity-child_1";

function parseToolJson(result: any): any {
  assert.ok(!result?.isError, `tool error: ${result?.content?.[0]?.text}`);
  const text = result.content[0].text;
  return JSON.parse(text);
}

type NamedArg = [string, { bytes: string; cl_type: unknown }];

function getNamedArgs(txWrapper: any): NamedArg[] {
  const named = txWrapper?.transaction?.payload?.fields?.args?.Named;
  assert.ok(Array.isArray(named), "expected payload.fields.args.Named");
  return named;
}

function getNamedArg(txWrapper: any, name: string): NamedArg[1] {
  const hit = getNamedArgs(txWrapper).find(([n]) => n === name);
  assert.ok(hit, `missing named arg ${name}`);
  return hit![1];
}

/** Decode Casper CL String bytes: u32 LE length + utf8. */
function decodeClStringBytes(hex: string): string {
  const buf = Buffer.from(hex, "hex");
  const len = buf.readUInt32LE(0);
  return buf.subarray(4, 4 + len).toString("utf8");
}

function assertEntryPoint(txWrapper: any, entryPoint: string) {
  const ep = getNamedArg(txWrapper, "entry_point");
  assert.strictEqual(decodeClStringBytes(ep.bytes), entryPoint);
}

/** Search length-prefixed utf8 needle inside hex blob (inner args List<U8>). */
function hexContainsUtf8(hex: string, needle: string): boolean {
  const n = Buffer.from(needle, "utf8").toString("hex");
  return hex.toLowerCase().includes(n.toLowerCase());
}

async function runTests() {
  console.log("=== Running MCP Write Parity Tests (Wave 4 G25/G26) ===");
  const registeredTools = (server as any)._registeredTools;
  const prevAdmin = process.env.MCP_ADMIN_TOKEN;
  const prevInternal = process.env.INTERNAL_SERVICE_KEY;

  try {
    process.env.MCP_ADMIN_TOKEN = ADMIN;
    delete process.env.INTERNAL_SERVICE_KEY;

    // ----------------------------------------------------
    // G26: create_task with parentTaskId
    // ----------------------------------------------------
    console.log("\n1. create_task + parentTaskId semantic parity...");
    {
      const withParent = await registeredTools["create_task"].handler({
        senderHex: SENDER,
        taskId: "parity-child-1",
        budgetMotes: "5000000000",
        metadataUri: "ipfs://meta",
        deadline: 999999,
        parentTaskId: "parent-abc",
      });
      const txWith = parseToolJson(withParent);
      assert.ok(txWith.transaction, "returns { transaction }");
      assertEntryPoint(txWith, "create_task");

      const innerArgs = getNamedArg(txWith, "args").bytes;
      assert.ok(
        hexContainsUtf8(innerArgs, "parent-abc"),
        "parent id must appear in inner args bytes"
      );
      assert.ok(
        hexContainsUtf8(innerArgs, "parent_task_id"),
        "parent_task_id arg name must be present"
      );
      assert.ok(
        hexContainsUtf8(innerArgs, "parity-child-1"),
        "task_id must be present"
      );

      // Outer attached amount present as named arg (budget is NOT an inner create_task field).
      const amountArg = getNamedArg(txWith, "amount");
      assert.ok(amountArg.bytes.length > 0, "outer amount present");
      console.log("  [PASS] G26: create_task encodes parentTaskId");

      const noParent = await registeredTools["create_task"].handler({
        senderHex: SENDER,
        taskId: "parity-child-2",
        budgetMotes: "5000000000",
        metadataUri: "ipfs://meta",
        deadline: 999999,
      });
      const txNone = parseToolJson(noParent);
      assertEntryPoint(txNone, "create_task");
      const innerNone = getNamedArg(txNone, "args").bytes;
      assert.ok(
        hexContainsUtf8(innerNone, "parent_task_id"),
        "None path still includes parent_task_id key"
      );
      assert.ok(
        !hexContainsUtf8(innerNone, "parent-abc"),
        "None path must not encode parent-abc"
      );
      assert.notStrictEqual(
        innerArgs,
        innerNone,
        "with/without parentTaskId must produce different inner args"
      );
      console.log("  [PASS] G26: create_task without parent differs (None path)");
    }

    // ----------------------------------------------------
    // G26: distribute_treasury happy-path args
    // ----------------------------------------------------
    console.log("\n2. distribute_treasury happy-path...");
    {
      const ok = await registeredTools["distribute_treasury"].handler({
        senderHex: SENDER,
        agentHex: AGENT,
        amountMotes: "2500000000",
        adminToken: ADMIN,
      });
      const tx = parseToolJson(ok);
      assertEntryPoint(tx, "distribute_treasury");
      const inner = getNamedArg(tx, "args").bytes;
      assert.ok(hexContainsUtf8(inner, "agent"), "agent arg name");
      assert.ok(hexContainsUtf8(inner, "amount"), "amount arg name");
      // amount 2500000000 = 0x9502f900 — encoded in U512 bytes; check name presence is enough + non-empty
      assert.ok(inner.length > 20, "inner args non-trivial");
      console.log("  [PASS] G26: distribute_treasury builds agent+amount args");

      const denied = await registeredTools["distribute_treasury"].handler({
        senderHex: SENDER,
        agentHex: AGENT,
        amountMotes: "2500000000",
        adminToken: "wrong-token",
      });
      assert.strictEqual(denied.isError, true);
      assert.ok(String(denied.content?.[0]?.text || "").includes("Unauthorized"));
      console.log("  [PASS] G26: distribute_treasury rejects wrong adminToken");

      const badHex = await registeredTools["distribute_treasury"].handler({
        senderHex: SENDER,
        agentHex: "not-a-key",
        amountMotes: "1000",
        adminToken: ADMIN,
      });
      assert.strictEqual(badHex.isError, true, "bad agentHex → isError");
      console.log("  [PASS] G26: distribute_treasury rejects bad agentHex");
    }

    // ----------------------------------------------------
    // G26: set_fee_rate happy-path admin token
    // ----------------------------------------------------
    console.log("\n3. set_fee_rate happy-path admin token...");
    {
      const ok = await registeredTools["set_fee_rate"].handler({
        senderHex: SENDER,
        feeBps: 500,
        adminToken: ADMIN,
      });
      const tx = parseToolJson(ok);
      assertEntryPoint(tx, "set_fee_rate");
      const inner = getNamedArg(tx, "args").bytes;
      assert.ok(hexContainsUtf8(inner, "fee_bps"), "fee_bps arg name");
      console.log("  [PASS] G26: set_fee_rate happy-path with valid adminToken");

      const denied = await registeredTools["set_fee_rate"].handler({
        senderHex: SENDER,
        feeBps: 500,
        adminToken: "nope",
      });
      assert.strictEqual(denied.isError, true);
      console.log("  [PASS] G26: set_fee_rate rejects wrong adminToken");
    }

    // ----------------------------------------------------
    // G25: MCP get_task_details exposes parent_task_id
    // ----------------------------------------------------
    console.log("\n4. MCP get_task_details parent_task_id read surface...");
    {
      await pool.query("DELETE FROM tasks WHERE id IN (?, ?)", [
        PARENT_ID,
        CHILD_ID,
      ]);
      await pool.query(
        `INSERT INTO tasks (
          id, creator_public_key, budget_motes, status, transaction_hash,
          domain, prompt, deadline, parent_task_id, timestamp
        ) VALUES (?, 'creator', 100, 'Open', 'tx-p', 'defi_analysis', 'p', 1, NULL, NOW())`,
        [PARENT_ID]
      );
      await pool.query(
        `INSERT INTO tasks (
          id, creator_public_key, budget_motes, status, transaction_hash,
          domain, prompt, deadline, parent_task_id, timestamp
        ) VALUES (?, 'creator', 100, 'Open', 'tx-c', 'defi_analysis', 'c', 1, ?, NOW())`,
        [CHILD_ID, PARENT_ID]
      );

      const result = await registeredTools["get_task_details"].handler({
        taskId: CHILD_ID,
      });
      assert.ok(!result.isError, result.content?.[0]?.text);
      const task = JSON.parse(result.content[0].text);
      assert.strictEqual(
        task.parent_task_id,
        PARENT_ID,
        "MCP read surface must include parent_task_id"
      );

      assert.ok(
        registeredTools["get_subtasks"],
        "get_subtasks tool must be registered (TECH_SPEC parity)"
      );
      const sub = await registeredTools["get_subtasks"].handler({
        parentTaskId: PARENT_ID,
      });
      assert.ok(!sub.isError, sub.content?.[0]?.text);
      const children = JSON.parse(sub.content[0].text);
      assert.ok(Array.isArray(children), "get_subtasks returns array");
      assert.strictEqual(children.length, 1, "one child for parent");
      assert.strictEqual(children[0].id, CHILD_ID);
      assert.strictEqual(children[0].parent_task_id, PARENT_ID);

      await pool.query("DELETE FROM tasks WHERE id IN (?, ?)", [
        PARENT_ID,
        CHILD_ID,
      ]);
      console.log("  [PASS] G25: get_task_details + get_subtasks parent_task_id surface");
    }

    console.log("\n=== ALL MCP WRITE PARITY TESTS PASSED ===");
  } finally {
    if (prevAdmin === undefined) delete process.env.MCP_ADMIN_TOKEN;
    else process.env.MCP_ADMIN_TOKEN = prevAdmin;
    if (prevInternal === undefined) delete process.env.INTERNAL_SERVICE_KEY;
    else process.env.INTERNAL_SERVICE_KEY = prevInternal;
    await pool.end().catch(() => undefined);
  }
}

runTests().catch((e) => {
  console.error("Test execution failed:", e);
  process.exit(1);
});

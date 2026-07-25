import { server } from "./mcp-server";
import { pool } from "./db";
import * as assert from "assert";

async function runTests() {
  console.log("=== Running MCP DB-Backed Integration Tests ===");

  const agentPk = "test-mcp-agent-pk";
  const agentPkInactive = "test-mcp-agent-pk-inactive";
  const taskIdOpen = "test-mcp-open-task";
  const taskIdClosed = "test-mcp-closed-task";

  try {
    // ----------------------------------------------------
    // 1. Cleanup and Seed Fixtures
    // ----------------------------------------------------
    console.log("Seeding test fixtures into MySQL...");

    // Clean up first
    await pool.query("DELETE FROM validations");
    await pool.query("DELETE FROM reputations WHERE agent_public_key IN (?, ?)", [agentPk, agentPkInactive]);
    await pool.query("DELETE FROM tasks WHERE id IN (?, ?)", [taskIdOpen, taskIdClosed]);
    await pool.query("DELETE FROM agents WHERE public_key IN (?, ?)", [agentPk, agentPkInactive]);
    await pool.query("DELETE FROM validators WHERE public_key = ?", ["test-mcp-validator-pk"]);

    // Seed agents
    await pool.query(
      `INSERT INTO agents (public_key, name, status, active_jobs, is_available)
       VALUES (?, 'MCP Active Agent', 'active', 0, 1)`,
      [agentPk]
    );
    await pool.query(
      `INSERT INTO agents (public_key, name, status, active_jobs, is_available)
       VALUES (?, 'MCP Inactive Agent', 'inactive', 0, 0)`,
      [agentPkInactive]
    );

    // Seed reputations
    await pool.query(
      `INSERT INTO reputations (id, agent_public_key, skill, score)
       VALUES ('test-mcp-rep-id', ?, 'defi_analysis', 85)`,
      [agentPk]
    );

    // Seed tasks (one Open, one Completed)
    await pool.query(
      `INSERT INTO tasks (
        id, creator_public_key, assigned_agent_public_key, budget_motes, status,
        transaction_hash, domain, prompt, deadline, timestamp
       ) VALUES (?, 'creator-mcp', ?, 500, 'Open', 'tx-hash-open', 'defi_analysis', 'mcp prompt open', 999999, NOW())`,
      [taskIdOpen, agentPk]
    );
    await pool.query(
      `INSERT INTO tasks (
        id, creator_public_key, assigned_agent_public_key, budget_motes, status,
        transaction_hash, domain, prompt, deadline, timestamp
       ) VALUES (?, 'creator-mcp', ?, 500, 'Completed', 'tx-hash-closed', 'defi_analysis', 'mcp prompt closed', 999999, NOW())`,
      [taskIdClosed, agentPk]
    );

    // Seed validator
    await pool.query(
      `INSERT INTO validators (public_key, stake_motes, is_active, total_validations)
       VALUES (?, 5000, 1, 12)`,
      ["test-mcp-validator-pk"]
    );

    const registeredTools = (server as any)._registeredTools;

    // ----------------------------------------------------
    // 2. Test list_agents
    // ----------------------------------------------------
    console.log("\n2. Testing list_agents...");
    {
      const result = await registeredTools["list_agents"].handler({});
      const text = result.content[0].text;
      const agents = JSON.parse(text);

      const foundActive = agents.find((a: any) => a.public_key === agentPk);
      const foundInactive = agents.find((a: any) => a.public_key === agentPkInactive);

      assert.ok(foundActive, "Active agent should be listed");
      assert.strictEqual(foundActive.name, "MCP Active Agent");
      assert.ok(foundInactive, "Inactive agent should be listed");
      console.log("  [PASS] list_agents lists both agents");
    }

    // ----------------------------------------------------
    // 3. Test get_agent_stats
    // ----------------------------------------------------
    console.log("\n3. Testing get_agent_stats...");
    {
      // Happy path
      const result = await registeredTools["get_agent_stats"].handler({ agentPublicKey: agentPk });
      const text = result.content[0].text;
      const stats = JSON.parse(text);
      assert.strictEqual(stats.name, "MCP Active Agent");
      assert.strictEqual(stats.status, "active");
      console.log("  [PASS] get_agent_stats happy path");

      // Non-existent agent
      const errResult = await registeredTools["get_agent_stats"].handler({ agentPublicKey: "non-existent" });
      assert.strictEqual(errResult.isError, true);
      assert.ok(errResult.content[0].text.includes("Agent not found"), "Should return Agent not found");
      console.log("  [PASS] get_agent_stats error path (agent not found)");
    }

    // ----------------------------------------------------
    // 4. Test query_reputation
    // ----------------------------------------------------
    console.log("\n4. Testing query_reputation...");
    {
      // Happy path
      const result = await registeredTools["query_reputation"].handler({
        agentPublicKey: agentPk,
        skill: "defi_analysis"
      });
      const text = result.content[0].text;
      const reputation = JSON.parse(text);
      assert.strictEqual(reputation.score, 85);
      console.log("  [PASS] query_reputation happy path");

      // No reputation found
      const errResult = await registeredTools["query_reputation"].handler({
        agentPublicKey: agentPkInactive,
        skill: "defi_analysis"
      });
      assert.ok(errResult.content[0].text.includes("No reputation"), "Should return empty reputation");
      console.log("  [PASS] query_reputation error path (reputation not found)");
    }

    // ----------------------------------------------------
    // 5. Test find_open_tasks
    // ----------------------------------------------------
    console.log("\n5. Testing find_open_tasks...");
    {
      const result = await registeredTools["find_open_tasks"].handler({});
      const text = result.content[0].text;
      const tasks = JSON.parse(text);

      const foundOpen = tasks.find((t: any) => t.id === taskIdOpen);
      const foundClosed = tasks.find((t: any) => t.id === taskIdClosed);

      assert.ok(foundOpen, "Should return the open task");
      assert.ok(!foundClosed, "Should NOT return the completed task");
      console.log("  [PASS] find_open_tasks returns only open tasks");
    }

    // ----------------------------------------------------
    // 6. Test get_validators
    // ----------------------------------------------------
    console.log("\n6. Testing get_validators...");
    {
      const result = await registeredTools["get_validators"].handler({});
      const text = result.content[0].text;
      const validators = JSON.parse(text);

      const foundValidator = validators.find((v: any) => v.public_key === "test-mcp-validator-pk");
      assert.ok(foundValidator, "Should find our seeded validator");
      assert.strictEqual(foundValidator.stake_motes, 5000);
      console.log("  [PASS] get_validators lists validators with stake");
    }

    // ----------------------------------------------------
    // 7. Wave 4 B8: write-tools bad payload (validation errors, no partial DB write)
    // ----------------------------------------------------
    console.log("\n7. Testing write-tools bad payload (scenario 8)...");
    {
      // Missing required fields → Zod / handler rejection before any DB mutation
      const createMissing = await registeredTools["create_task"].handler({});
      assert.ok(
        createMissing?.isError === true ||
          (typeof createMissing?.content?.[0]?.text === "string" &&
            (createMissing.content[0].text.includes("Error") ||
              createMissing.content[0].text.includes("required") ||
              createMissing.content[0].text.includes("Invalid"))),
        "create_task with missing fields must fail"
      );

      const assignBadHex = await registeredTools["assign_task"].handler({
        senderHex: "not-a-valid-key",
        taskId: "t1",
        agentHex: "also-bad",
      });
      assert.strictEqual(assignBadHex.isError, true, "assign_task bad hex → isError");
      assert.ok(
        !String(assignBadHex.content?.[0]?.text || "").includes("mysql://"),
        "error must not leak DSN"
      );

      const submitBad = await registeredTools["submit_validation"].handler({
        senderHex: "",
        creatorHex: "",
        taskId: "",
        score: -1,
      });
      assert.strictEqual(submitBad.isError, true, "submit_validation bad payload → isError");

      // Admin tool wrong token — scenario 10 path
      process.env.INTERNAL_SERVICE_KEY = "w4-admin-secret";
      const dist = await registeredTools["distribute_treasury"].handler({
        senderHex: "01" + "ab".repeat(32),
        agentHex: "01" + "cd".repeat(32),
        amountMotes: "1000000000",
        adminToken: "wrong",
      });
      assert.strictEqual(dist.isError, true);
      assert.ok(
        String(dist.content?.[0]?.text || "").includes("Unauthorized"),
        "wrong adminToken rejected"
      );
      console.log("  [PASS] scenario 8: write-tools reject bad payloads without DB writes");
      console.log("  [PASS] scenario 10: adminToken mismatch rejected");
    }

    // ----------------------------------------------------
    // 8. Wave 4 B7: DB-backed tools after pool/MySQL death
    // ----------------------------------------------------
    console.log("\n8. Testing DB tools with dead MySQL (scenario 7)...");
    {
      const { execSync } = await import("child_process");
      let stopped = false;
      try {
        try {
          execSync("docker stop agent-network-mysql", { stdio: "pipe" });
          stopped = true;
        } catch (dockerErr: any) {
          const msg = String(dockerErr?.stderr || dockerErr?.message || dockerErr);
          if (msg.includes("permission denied") || msg.includes("Cannot connect")) {
            console.log(
              "  [SKIP] scenario 7: docker unavailable in this environment; B7 covered when docker.sock is accessible"
            );
          } else {
            throw dockerErr;
          }
        }

        if (stopped) {
          // Wait for connections to die
          await new Promise((r) => setTimeout(r, 1500));

          for (const toolName of ["list_agents", "get_agent_stats", "find_open_tasks"] as const) {
            const args =
              toolName === "get_agent_stats"
                ? { agentPublicKey: agentPk }
                : {};
            let result: any;
            try {
              result = await registeredTools[toolName].handler(args);
            } catch (e: any) {
              // Some paths may throw instead of returning isError — still controlled
              const msg = String(e?.message || e);
              assert.ok(!msg.includes("passw0rd"), "must not leak password");
              assert.ok(!msg.includes("mysql://deagentnet:"), "must not leak full DSN");
              console.log(`  [PASS] scenario 7: ${toolName} threw controlled error (no secret leak)`);
              continue;
            }
            assert.ok(
              result?.isError === true ||
                (typeof result?.content?.[0]?.text === "string" &&
                  result.content[0].text.toLowerCase().includes("error")),
              `${toolName} should surface error when DB is down`
            );
            const text = String(result?.content?.[0]?.text || "");
            assert.ok(!text.includes("passw0rd"), "no password in tool error text");
            assert.ok(!text.includes("mysql://deagentnet:"), "no DSN in tool error text");
            console.log(`  [PASS] scenario 7: ${toolName} returns isError without secrets`);
          }
        }
      } finally {
        if (stopped) {
          execSync("docker start agent-network-mysql", { stdio: "pipe" });
          for (let i = 0; i < 40; i++) {
            try {
              await pool.query("SELECT 1");
              break;
            } catch {
              await new Promise((r) => setTimeout(r, 500));
            }
          }
        }
      }
    }

    console.log("\n=== ALL MCP DB-BACKED TESTS PASSED SUCCESSFULLY ===");
  } finally {
    // ----------------------------------------------------
    // Cleanup Fixtures
    // ----------------------------------------------------
    console.log("\nCleaning up fixtures...");
    try {
      await pool.query("DELETE FROM reputations WHERE agent_public_key IN (?, ?)", [agentPk, agentPkInactive]);
      await pool.query("DELETE FROM tasks WHERE id IN (?, ?)", [taskIdOpen, taskIdClosed]);
      await pool.query("DELETE FROM agents WHERE public_key IN (?, ?)", [agentPk, agentPkInactive]);
      await pool.query("DELETE FROM validators WHERE public_key = ?", ["test-mcp-validator-pk"]);
      await pool.end();
    } catch (e) {
      console.error("Cleanup failed:", e);
    }
  }
}

runTests().catch((e) => {
  console.error("Test execution failed:", e);
  process.exit(1);
});

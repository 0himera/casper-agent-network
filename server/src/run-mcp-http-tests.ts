/**
 * Wave 4 B6 / B10 (SSE path): HTTP-level MCP transport tests.
 * Uses createApp() without binding a long-lived process.
 */
import http from "http";
import { createApp, resetRateLimiters } from "./mcp-server";
import * as assert from "assert";

function request(
  server: http.Server,
  method: string,
  path: string,
  opts: { headers?: Record<string, string>; body?: string } = {}
): Promise<{ status: number; body: string }> {
  return new Promise((resolve, reject) => {
    const addr = server.address();
    if (!addr || typeof addr === "string") {
      reject(new Error("server not listening"));
      return;
    }
    const req = http.request(
      {
        host: "127.0.0.1",
        port: addr.port,
        method,
        path,
        headers: {
          "content-type": "application/json",
          ...(opts.headers || {}),
        },
      },
      (res) => {
        const chunks: Buffer[] = [];
        res.on("data", (c) => chunks.push(c));
        res.on("end", () => {
          resolve({
            status: res.statusCode || 0,
            body: Buffer.concat(chunks).toString("utf8"),
          });
        });
      }
    );
    req.on("error", reject);
    if (opts.body) req.write(opts.body);
    req.end();
  });
}

async function runTests() {
  console.log("=== Running MCP HTTP / SSE Transport Tests (Wave 4 B6/B10) ===");
  resetRateLimiters();

  const origKey = process.env.INTERNAL_SERVICE_KEY;
  const origAdmin = process.env.MCP_ADMIN_TOKEN;

  try {
    // ----------------------------------------------------
    // B6: POST /message without SSE session → 400
    // ----------------------------------------------------
    {
      process.env.INTERNAL_SERVICE_KEY = "w4-http-secret";
      const app = createApp();
      const server = http.createServer(app);
      await new Promise<void>((r) => server.listen(0, "127.0.0.1", () => r()));

      const res = await request(server, "POST", "/message", {
        headers: { Authorization: "Bearer w4-http-secret" },
        body: JSON.stringify({ jsonrpc: "2.0", method: "tools/list", id: 1 }),
      });

      assert.strictEqual(res.status, 400, "B6: expected 400");
      assert.ok(
        res.body.includes("SSE connection not established"),
        `B6: unexpected body: ${res.body}`
      );
      console.log("  [PASS] scenario 6: /message without SSE returns 400");

      await new Promise<void>((r) => server.close(() => r()));
    }

    // ----------------------------------------------------
    // B10: SSE query token required when key is set
    // ----------------------------------------------------
    {
      process.env.INTERNAL_SERVICE_KEY = "w4-http-secret";
      delete process.env.MCP_ADMIN_TOKEN;
      const app = createApp();
      const server = http.createServer(app);
      await new Promise<void>((r) => server.listen(0, "127.0.0.1", () => r()));

      // Bearer alone is not enough for /sse — needs ?token=
      // But authMiddleware also requires Bearer first.
      const noToken = await request(server, "GET", "/sse", {
        headers: { Authorization: "Bearer w4-http-secret" },
      });
      assert.strictEqual(noToken.status, 401, "B10: /sse without query token → 401");
      assert.ok(
        noToken.body.includes("Unauthorized"),
        `B10: unexpected body: ${noToken.body}`
      );
      console.log("  [PASS] scenario 10: /sse rejects missing query token in prod-mode");

      // Fallback mode: no INTERNAL_SERVICE_KEY → middleware allows; /sse also open
      delete process.env.INTERNAL_SERVICE_KEY;
      const app2 = createApp();
      const server2 = http.createServer(app2);
      await new Promise<void>((r) => server2.listen(0, "127.0.0.1", () => r()));

      const health = await request(server2, "GET", "/health");
      assert.strictEqual(health.status, 200, "health ok in fallback");
      console.log("  [PASS] scenario 10: fallback mode allows /health without key");

      await new Promise<void>((r) => server.close(() => r()));
      await new Promise<void>((r) => server2.close(() => r()));
    }

    console.log("\n=== ALL MCP HTTP TESTS PASSED ===");
  } finally {
    if (origKey === undefined) delete process.env.INTERNAL_SERVICE_KEY;
    else process.env.INTERNAL_SERVICE_KEY = origKey;
    if (origAdmin === undefined) delete process.env.MCP_ADMIN_TOKEN;
    else process.env.MCP_ADMIN_TOKEN = origAdmin;
  }
}

runTests().catch((e) => {
  console.error("Test failed:", e);
  process.exit(1);
});

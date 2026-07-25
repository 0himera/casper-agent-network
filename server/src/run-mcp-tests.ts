import {
  rateLimiter,
  authMiddleware,
  getRateLimiterSizes,
  resetRateLimiters,
} from "./mcp-server";
import * as assert from "assert";

// Mock Express Response
class MockResponse {
  statusCode: number = 200;
  headers: Record<string, string> = {};
  body: any = null;

  status(code: number) {
    this.statusCode = code;
    return this;
  }

  json(data: any) {
    this.body = data;
    return this;
  }

  send(data: any) {
    this.body = data;
    return this;
  }
}

// Mock Next Function
function createNext() {
  let called = false;
  const fn = () => {
    called = true;
  };
  fn.isCalled = () => called;
  return fn;
}

async function runTests() {
  console.log("=== Running MCP Server Middleware Tests ===");

  // Save original env vars
  const origServiceKey = process.env.INTERNAL_SERVICE_KEY;

  try {
    // ----------------------------------------------------
    // 1. authMiddleware Tests
    // ----------------------------------------------------
    console.log("\n1. Running authMiddleware Tests...");

    // Test 1.1: /health always skips auth
    {
      process.env.INTERNAL_SERVICE_KEY = "test_secret_key";
      const req: any = { path: "/health", headers: {} };
      const res = new MockResponse();
      const next = createNext();

      authMiddleware(req, res as any, next);

      assert.strictEqual(next.isCalled(), true, "Should call next() for /health without token");
      assert.strictEqual(res.statusCode, 200, "Should have status 200");
      console.log("  [PASS] Test 1.1: /health skips auth");
    }

    // Test 1.2: Missing Authorization Header
    {
      process.env.INTERNAL_SERVICE_KEY = "test_secret_key";
      const req: any = { path: "/api/agents", headers: {} };
      const res = new MockResponse();
      const next = createNext();

      authMiddleware(req, res as any, next);

      assert.strictEqual(next.isCalled(), false, "Should not call next()");
      assert.strictEqual(res.statusCode, 401, "Should return 401 Unauthorized");
      assert.deepStrictEqual(res.body, { error: "Unauthorized. Valid Bearer token required." });
      console.log("  [PASS] Test 1.2: Missing Authorization header rejected with 401");
    }

    // Test 1.3: Invalid Bearer Token
    {
      process.env.INTERNAL_SERVICE_KEY = "test_secret_key";
      const req: any = { path: "/api/agents", headers: { authorization: "Bearer wrong_key" } };
      const res = new MockResponse();
      const next = createNext();

      authMiddleware(req, res as any, next);

      assert.strictEqual(next.isCalled(), false, "Should not call next()");
      assert.strictEqual(res.statusCode, 401, "Should return 401 Unauthorized");
      console.log("  [PASS] Test 1.3: Invalid Bearer token rejected with 401");
    }

    // Test 1.4: Valid Bearer Token (Happy Path)
    {
      process.env.INTERNAL_SERVICE_KEY = "test_secret_key";
      const req: any = { path: "/api/agents", headers: { authorization: "Bearer test_secret_key" } };
      const res = new MockResponse();
      const next = createNext();

      authMiddleware(req, res as any, next);

      assert.strictEqual(next.isCalled(), true, "Should call next()");
      assert.strictEqual(res.statusCode, 200, "Should remain 200 OK");
      console.log("  [PASS] Test 1.4: Valid Bearer token authorized successfully");
    }

    // Test 1.5: No env key configured (Public/fallback mode) — scenario 10
    {
      delete process.env.INTERNAL_SERVICE_KEY;
      const req: any = { path: "/api/agents", headers: {} };
      const res = new MockResponse();
      const next = createNext();

      authMiddleware(req, res as any, next);

      assert.strictEqual(next.isCalled(), true, "Should call next() when INTERNAL_SERVICE_KEY is unset");
      console.log("  [PASS] Test 1.5 / scenario 10: Fallback mode (no env key) allows requests");
    }

    // Test 1.6: Prod-mode rejects bypass without Bearer — scenario 10
    {
      process.env.INTERNAL_SERVICE_KEY = "prod_secret";
      const req: any = { path: "/sse", headers: {} };
      const res = new MockResponse();
      const next = createNext();
      authMiddleware(req, res as any, next);
      assert.strictEqual(next.isCalled(), false);
      assert.strictEqual(res.statusCode, 401);
      console.log("  [PASS] scenario 10: prod-mode rejects missing Bearer (no bypass)");
    }

    // ----------------------------------------------------
    // 2. rateLimiter Tests
    // ----------------------------------------------------
    console.log("\n2. Running rateLimiter Tests...");
    resetRateLimiters();

    // Test 2.1: Read requests rate limiting (Happy Path up to 60)
    {
      const req: any = {
        headers: {},
        socket: { remoteAddress: "192.168.1.1" },
        body: { method: "tools/list" }
      };
      
      // Let's call it 60 times
      for (let i = 0; i < 60; i++) {
        const res = new MockResponse();
        const next = createNext();
        rateLimiter(req, res as any, next);
        assert.strictEqual(next.isCalled(), true, `Read request #${i + 1} should be allowed`);
        assert.strictEqual(res.statusCode, 200);
      }

      // 61st read request should be blocked
      const res = new MockResponse();
      const next = createNext();
      rateLimiter(req, res as any, next);
      assert.strictEqual(next.isCalled(), false, "61st read request should be blocked");
      assert.strictEqual(res.statusCode, 429, "61st read request should return 429");
      assert.ok(res.body.error.includes("Too many requests for read operations"), "Should have correct error message");
      console.log("  [PASS] Test 2.1: Read requests blocked after 60 requests");
    }

    // Test 2.2: Write requests rate limiting (Happy Path up to 10)
    {
      const req: any = {
        headers: {},
        socket: { remoteAddress: "192.168.1.2" },
        body: {
          method: "tools/call",
          params: { name: "create_task" }
        }
      };

      // Let's call it 10 times
      for (let i = 0; i < 10; i++) {
        const res = new MockResponse();
        const next = createNext();
        rateLimiter(req, res as any, next);
        assert.strictEqual(next.isCalled(), true, `Write request #${i + 1} should be allowed`);
        assert.strictEqual(res.statusCode, 200);
      }

      // 11th write request should be blocked
      const res = new MockResponse();
      const next = createNext();
      rateLimiter(req, res as any, next);
      assert.strictEqual(next.isCalled(), false, "11th write request should be blocked");
      assert.strictEqual(res.statusCode, 429, "11th write request should return 429");
      assert.ok(res.body.error.includes("Too many requests for write operations"), "Should have correct error message");
      console.log("  [PASS] Test 2.2: Write requests blocked after 10 requests");
    }

    // Test 2.3 / scenario 9: many unique IPs — Map stays bounded via eviction/cap (B9).
    {
      resetRateLimiters();
      const before = getRateLimiterSizes();
      assert.strictEqual(before.read, 0);

      for (let i = 0; i < 2500; i++) {
        const req: any = {
          headers: { "x-forwarded-for": `10.9.${Math.floor(i / 256)}.${i % 256}` },
          socket: { remoteAddress: "127.0.0.1" },
          body: { method: "tools/list" },
        };
        const res = new MockResponse();
        const next = createNext();
        rateLimiter(req, res as any, next);
      }

      const after = getRateLimiterSizes();
      assert.ok(after.read <= 1024, `Map must stay bounded (<=1024), got ${after.read}`);
      assert.ok(after.read < 2500, "eviction must drop older IP keys under unique-IP flood");
      console.log(
        `  [PASS] scenario 9: rateLimiter Map bounded after unique-IP flood (size=${after.read})`
      );
      resetRateLimiters();
    }

    console.log("\n=== ALL MCP TESTS PASSED SUCCESSFULLY ===");
  } finally {
    // Restore original env
    process.env.INTERNAL_SERVICE_KEY = origServiceKey;
  }
}

runTests().catch((e) => {
  console.error("Test failed:", e);
  process.exit(1);
});

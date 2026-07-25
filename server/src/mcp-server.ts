import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { SSEServerTransport } from "@modelcontextprotocol/sdk/server/sse.js";
import express from "express";
import { z } from "zod";
import { pool } from './db';
import { RowDataPacket } from 'mysql2';
import { config } from './config';
import fs from 'fs';
import path from 'path';
import {
  Args,
  CLTypeUInt8,
  CLTypeString,
  CLValue,
  Hash,
  PublicKey,
  SessionBuilder,
  Key,
  RpcClient,
  HttpHandler,
  Transaction
} from 'casper-js-sdk';

export const server = new McpServer({
  name: "casper-agent-network",
  version: "1.0.0",
});

// Public task allowlist — never expose exam_templates / exam_assignments fields.
const TASK_PUBLIC_COLUMNS = `
  id, creator_public_key, assigned_agent_public_key, budget_motes, status,
  result_hash, result, metadata_uri, transaction_hash, domain, skill_id,
  prompt, deadline, result_signature, validator_audit, timestamp, parent_task_id
`.trim();

type PublicTaskRow = {
  id: string;
  creator_public_key: string;
  assigned_agent_public_key: string | null;
  budget_motes: number;
  status: string;
  result_hash: string | null;
  result: string | null;
  metadata_uri: string | null;
  transaction_hash: string;
  domain: string;
  skill_id: string | null;
  prompt: string;
  deadline: number;
  result_signature: string | null;
  validator_audit: unknown;
  timestamp: Date;
  parent_task_id: string | null;
};

function toPublicTask(row: RowDataPacket): PublicTaskRow {
  return {
    id: row.id,
    creator_public_key: row.creator_public_key,
    assigned_agent_public_key: row.assigned_agent_public_key ?? null,
    budget_motes: Number(row.budget_motes),
    status: row.status,
    result_hash: row.result_hash ?? null,
    result: row.result ?? null,
    metadata_uri: row.metadata_uri ?? null,
    transaction_hash: row.transaction_hash,
    domain: row.domain,
    skill_id: row.skill_id ?? null,
    prompt: row.prompt,
    deadline: Number(row.deadline),
    result_signature: row.result_signature ?? null,
    validator_audit: row.validator_audit ?? null,
    timestamp: row.timestamp,
    parent_task_id: row.parent_task_id ?? null,
  };
}

// Helper to load local proxy wasm
const getProxyWasm = (): Uint8Array => {
  const buffer = fs.readFileSync(path.resolve(__dirname, `./resources/proxy_caller.wasm`));
  return new Uint8Array(buffer);
};

// Builder for transactions to return to agents for signing
const buildContractTransaction = (
  senderHex: string,
  entryPoint: string,
  innerArgsMap: Record<string, CLValue>,
  attachedMotes: string = '0'
): any => {
  const contractWasm = getProxyWasm();
  const packageHash = config.contractPackageHash;
  if (!packageHash) {
    throw new Error("Missing contractPackageHash in config");
  }

  const innerArgs = Args.fromMap(innerArgsMap);

  const serializedArgs = CLValue.newCLList(
    CLTypeUInt8,
    Array.from(innerArgs.toBytes()).map((value) => CLValue.newCLUint8(value))
  );

  const args = Args.fromMap({
    amount: CLValue.newCLUInt512(attachedMotes),
    attached_value: CLValue.newCLUInt512(attachedMotes),
    entry_point: CLValue.newCLString(entryPoint),
    package_hash: CLValue.newCLByteArray(Hash.fromHex(packageHash).toBytes()),
    args: serializedArgs
  });

  const payment = 15000000000; // 15 CSPR for complex calls

  const sessionTransaction = new SessionBuilder()
    .from(PublicKey.fromHex(senderHex))
    .runtimeArgs(args)
    .wasm(contractWasm)
    .payment(payment)
    .chainName('casper-test')
    .build();

  return {
    transaction: sessionTransaction.toJSON()
  };
};

// 1. list_agents
server.tool(
  "list_agents",
  {},
  async () => {
    const [agents] = await pool.query('SELECT * FROM agents ORDER BY timestamp DESC');
    return {
      content: [{ type: "text", text: JSON.stringify(agents, null, 2) }]
    };
  }
);

// 2. get_agent_stats
server.tool(
  "get_agent_stats",
  {
    agentPublicKey: z.string().describe("Casper public key of the agent"),
  },
  async ({ agentPublicKey }) => {
    const [rows] = await pool.query<RowDataPacket[]>('SELECT * FROM agents WHERE public_key = ?', [agentPublicKey]);
    const agent = rows[0];
    if (!agent) {
      return {
        content: [{ type: "text", text: `Agent not found: ${agentPublicKey}` }],
        isError: true
      };
    }
    return {
      content: [{ type: "text", text: JSON.stringify(agent, null, 2) }]
    };
  }
);

// 3. query_reputation
server.tool(
  "query_reputation",
  {
    agentPublicKey: z.string().describe("Casper public key of the agent"),
    skill: z.string().describe("Skill domain name (e.g. defi_analysis, code_review)"),
  },
  async ({ agentPublicKey, skill }) => {
    const [rows] = await pool.query<RowDataPacket[]>('SELECT * FROM reputations WHERE agent_public_key = ? AND skill = ?', [agentPublicKey, skill]);
    const reputation = rows[0];
    if (!reputation) {
      return {
        content: [{ type: "text", text: `No reputation score for agent ${agentPublicKey} on skill ${skill}` }]
      };
    }
    return {
      content: [{ type: "text", text: JSON.stringify(reputation, null, 2) }]
    };
  }
);

// 4. get_leaderboard
server.tool(
  "get_leaderboard",
  {
    domain: z.string().optional().describe("Optional skill domain filter"),
  },
  async ({ domain }) => {
    let items;
    if (domain) {
      [items] = await pool.query('SELECT * FROM reputations WHERE skill = ? ORDER BY score DESC', [domain]);
    } else {
      [items] = await pool.query('SELECT * FROM reputations ORDER BY score DESC');
    }
    return {
      content: [{ type: "text", text: JSON.stringify(items, null, 2) }]
    };
  }
);

// 5. find_open_tasks
server.tool(
  "find_open_tasks",
  {},
  async () => {
    const [tasks] = await pool.query<RowDataPacket[]>(
      `SELECT ${TASK_PUBLIC_COLUMNS} FROM tasks WHERE status = "Open" ORDER BY timestamp DESC`
    );
    const publicTasks = tasks.map(toPublicTask);
    return {
      content: [{ type: "text", text: JSON.stringify(publicTasks, null, 2) }]
    };
  }
);

// 6. create_task (Returns unsigned deploy JSON)
server.tool(
  "create_task",
  {
    senderHex: z.string().describe("Casper public key of sender/creator"),
    taskId: z.string().describe("Unique string identifier for task"),
    budgetMotes: z.string().describe("Amount of CSPR to escrow (in motes, e.g. 5000000000 for 5 CSPR)"),
    metadataUri: z.string().describe("Off-chain task description URI"),
    deadline: z.number().describe("Unix timestamp deadline for task execution"),
    parentTaskId: z.string().optional().describe("Optional parent task id for A2A child tasks"),
  },
  async ({ senderHex, taskId, budgetMotes, metadataUri, deadline, parentTaskId }) => {
    try {
      const tx = buildContractTransaction(senderHex, 'create_task', {
        task_id: CLValue.newCLString(taskId),
        metadata_uri: CLValue.newCLString(metadataUri),
        deadline: CLValue.newCLUint64(deadline),
        parent_task_id: CLValue.newCLOption(
          parentTaskId ? CLValue.newCLString(parentTaskId) : null,
          CLTypeString
        )
      }, budgetMotes);
      return {
        content: [{ type: "text", text: JSON.stringify(tx, null, 2) }]
      };
    } catch (err: any) {
      return {
        content: [{ type: "text", text: `Error: ${err.message}` }],
        isError: true
      };
    }
  }
);

// 7. assign_task
server.tool(
  "assign_task",
  {
    senderHex: z.string().describe("Casper public key of sender (creator)"),
    taskId: z.string().describe("Unique string identifier for task"),
    agentHex: z.string().describe("Casper public key of assigned agent"),
  },
  async ({ senderHex, taskId, agentHex }) => {
    try {
      const agentKeyStr = PublicKey.fromHex(agentHex).accountHash().toPrefixedString();
      const agentKey = Key.newKey(agentKeyStr);

      const tx = buildContractTransaction(senderHex, 'assign_task', {
        task_id: CLValue.newCLString(taskId),
        agent: CLValue.newCLKey(agentKey)
      });
      return {
        content: [{ type: "text", text: JSON.stringify(tx, null, 2) }]
      };
    } catch (err: any) {
      return {
        content: [{ type: "text", text: `Error: ${err.message}` }],
        isError: true
      };
    }
  }
);

// 8. update_agent_price
server.tool(
  "update_agent_price",
  {
    senderHex: z.string().describe("Casper public key of calling agent"),
    priceMotes: z.string().describe("Custom agent price in motes"),
  },
  async ({ senderHex, priceMotes }) => {
    try {
      const tx = buildContractTransaction(senderHex, 'set_price', {
        price: CLValue.newCLUInt512(priceMotes)
      });
      return {
        content: [{ type: "text", text: JSON.stringify(tx, null, 2) }]
      };
    } catch (err: any) {
      return {
        content: [{ type: "text", text: `Error: ${err.message}` }],
        isError: true
      };
    }
  }
);

// 9. register_agent_profile
server.tool(
  "register_agent_profile",
  {
    senderHex: z.string().describe("Casper public key of registering agent"),
    name: z.string().describe("Display name for agent"),
    description: z.string().describe("Capabilities description"),
    metadataUri: z.string().describe("Agent off-chain metadata link"),
  },
  async ({ senderHex, name, description, metadataUri }) => {
    try {
      // Sanitize non-ASCII characters to prevent LeftOverBytes in casper-js-sdk string serialization
      const sanitizeStr = (s: string) => s.replace(/[^\x00-\x7F]/g, '-');

      const tx = buildContractTransaction(senderHex, 'register_agent', {
        name: CLValue.newCLString(sanitizeStr(name)),
        description: CLValue.newCLString(sanitizeStr(description)),
        metadata_uri: CLValue.newCLString(sanitizeStr(metadataUri))
      });
      return {
        content: [{ type: "text", text: JSON.stringify(tx, null, 2) }]
      };
    } catch (err: any) {
      return {
        content: [{ type: "text", text: `Error: ${err.message}` }],
        isError: true
      };
    }
  }
);

// 10. submit_execution_result
server.tool(
  "submit_execution_result",
  {
    senderHex: z.string().describe("Casper public key of calling agent (must be assigned agent)"),
    creatorHex: z.string().describe("Casper public key of the task creator (tasks are namespaced by creator)"),
    taskId: z.string().describe("Task ID"),
    resultHash: z.string().describe("SHA-256 result hash"),
  },
  async ({ senderHex, creatorHex, taskId, resultHash }) => {
    try {
      const creatorKeyStr = PublicKey.fromHex(creatorHex).accountHash().toPrefixedString();
      const creatorKey = Key.newKey(creatorKeyStr);

      const tx = buildContractTransaction(senderHex, 'submit_result', {
        creator: CLValue.newCLKey(creatorKey),
        task_id: CLValue.newCLString(taskId),
        result_hash: CLValue.newCLString(resultHash)
      });
      return {
        content: [{ type: "text", text: JSON.stringify(tx, null, 2) }]
      };
    } catch (err: any) {
      return {
        content: [{ type: "text", text: `Error: ${err.message}` }],
        isError: true
      };
    }
  }
);

// 11. get_signing_instructions
server.tool(
  "get_signing_instructions",
  {},
  async () => {
    const instructions = `
# How to Sign Casper v5 Transactions Locally

To interact with the Casper Agent Network securely, you must sign transactions locally using your private PEM key before broadcasting.

### Node.js Example

\`\`\`javascript
import { Transaction, PrivateKey } from 'casper-js-sdk';
import fs from 'fs';

// 1. Get the unsigned transaction JSON from MCP (e.g. from \`submit_execution_result\`)
const unsignedTxJson = /* get transaction JSON */;

// 2. Load the transaction object
const transaction = Transaction.fromJSON(unsignedTxJson);

// 3. Load your private key from PEM file
const privateKeyPem = fs.readFileSync('./keys/secret_key.pem', 'utf8');
const privateKey = PrivateKey.fromPem(privateKeyPem);

// 4. Sign the transaction locally
transaction.sign(privateKey);

// 5. Export the signed transaction JSON to send to broadcast_transaction tool
const signedTxJson = transaction.toJSON();
\`\`\`

Once signed, call the \`broadcast_transaction\` tool in this MCP server to submit it to the blockchain.
`;
    return {
      content: [{ type: "text", text: instructions }]
    };
  }
);

// 12. broadcast_transaction
server.tool(
  "broadcast_transaction",
  {
    signedTransaction: z.any().describe("The JSON object of the signed Casper transaction"),
  },
  async ({ signedTransaction }) => {
    try {
      const transaction = Transaction.fromJSON(signedTransaction);
      const rpcHandler = new HttpHandler(config.nodeUrl);
      const rpcClient = new RpcClient(rpcHandler);
      
      const result = await rpcClient.putTransaction(transaction);
      return {
        content: [{ 
          type: "text", 
          text: JSON.stringify({
            success: true,
            transactionHash: result.transactionHash
          }, null, 2)
        }]
      };
    } catch (err: any) {
      return {
        content: [{ type: "text", text: `Error broadcasting transaction: ${err.message}` }],
        isError: true
      };
    }
  }
);

// 13. get_task_details
server.tool(
  "get_task_details",
  {
    taskId: z.string().describe("Task ID to fetch details for"),
  },
  async ({ taskId }) => {
    try {
      const [rows] = await pool.query<RowDataPacket[]>(
        `SELECT ${TASK_PUBLIC_COLUMNS} FROM tasks WHERE id = ?`,
        [taskId]
      );
      const task = rows[0];
      if (!task) {
        return {
          content: [{ type: "text", text: `Task not found: ${taskId}` }],
          isError: true
        };
      }
      return {
        content: [{ type: "text", text: JSON.stringify(toPublicTask(task), null, 2) }]
      };
    } catch (err: any) {
      return {
        content: [{ type: "text", text: `Error fetching task: ${err.message}` }],
        isError: true
      };
    }
  }
);

// 14. get_assigned_tasks
server.tool(
  "get_assigned_tasks",
  {
    agentPublicKey: z.string().describe("Public key of the agent to fetch tasks for"),
  },
  async ({ agentPublicKey }) => {
    try {
      const [tasks] = await pool.query<RowDataPacket[]>(
        `SELECT ${TASK_PUBLIC_COLUMNS} FROM tasks WHERE assigned_agent_public_key = ? AND status = "InProgress" ORDER BY timestamp DESC`,
        [agentPublicKey]
      );
      // Filter out tasks that already have a result submitted
      const activeTasks = tasks.filter(t => !t.result_hash).map(toPublicTask);
      return {
        content: [{ type: "text", text: JSON.stringify(activeTasks, null, 2) }]
      };
    } catch (err: any) {
      return {
        content: [{ type: "text", text: `Error fetching assigned tasks: ${err.message}` }],
        isError: true
      };
    }
  }
);

// 14b. get_subtasks — read-only filter by parent_task_id (A2A child listing; no orchestration).
server.tool(
  "get_subtasks",
  {
    parentTaskId: z.string().describe("Parent task id whose children to list"),
  },
  async ({ parentTaskId }) => {
    try {
      const [tasks] = await pool.query<RowDataPacket[]>(
        `SELECT ${TASK_PUBLIC_COLUMNS} FROM tasks WHERE parent_task_id = ? ORDER BY timestamp DESC`,
        [parentTaskId]
      );
      const publicTasks = tasks.map(toPublicTask);
      return {
        content: [{ type: "text", text: JSON.stringify(publicTasks, null, 2) }]
      };
    } catch (err: any) {
      return {
        content: [{ type: "text", text: `Error fetching subtasks: ${err.message}` }],
        isError: true
      };
    }
  }
);

// 15. update_agent_profile
server.tool(
  "update_agent_profile",
  {
    senderHex: z.string().describe("Casper public key of the agent"),
    name: z.string().describe("New display name"),
    description: z.string().describe("New capabilities description"),
    metadataUri: z.string().describe("New metadata URI"),
  },
  async ({ senderHex, name, description, metadataUri }) => {
    try {
      const sanitizeStr = (s: string) => s.replace(/[^\x00-\x7F]/g, '-');
      const tx = buildContractTransaction(senderHex, 'update_agent', {
        name: CLValue.newCLString(sanitizeStr(name)),
        description: CLValue.newCLString(sanitizeStr(description)),
        metadata_uri: CLValue.newCLString(sanitizeStr(metadataUri))
      });
      return {
        content: [{ type: "text", text: JSON.stringify(tx, null, 2) }]
      };
    } catch (err: any) {
      return {
        content: [{ type: "text", text: `Error: ${err.message}` }],
        isError: true
      };
    }
  }
);

// 16. set_availability
server.tool(
  "set_availability",
  {
    senderHex: z.string().describe("Casper public key of the agent"),
    available: z.boolean().describe("Whether the agent is available for new tasks"),
  },
  async ({ senderHex, available }) => {
    try {
      const tx = buildContractTransaction(senderHex, 'set_availability', {
        available: CLValue.newCLValueBool(available)
      });
      return {
        content: [{ type: "text", text: JSON.stringify(tx, null, 2) }]
      };
    } catch (err: any) {
      return {
        content: [{ type: "text", text: `Error: ${err.message}` }],
        isError: true
      };
    }
  }
);

// 17. increase_budget
server.tool(
  "increase_budget",
  {
    senderHex: z.string().describe("Casper public key of the task creator"),
    taskId: z.string().describe("Task ID to increase budget for"),
    additionalMotes: z.string().describe("Additional budget in motes"),
  },
  async ({ senderHex, taskId, additionalMotes }) => {
    try {
      const tx = buildContractTransaction(senderHex, 'increase_budget', {
        task_id: CLValue.newCLString(taskId)
      }, additionalMotes);
      return {
        content: [{ type: "text", text: JSON.stringify(tx, null, 2) }]
      };
    } catch (err: any) {
      return {
        content: [{ type: "text", text: `Error: ${err.message}` }],
        isError: true
      };
    }
  }
);

// 18. dispute_task
server.tool(
  "dispute_task",
  {
    senderHex: z.string().describe("Casper public key of the disputer (creator or admin)"),
    creatorHex: z.string().describe("Casper public key of the task creator"),
    taskId: z.string().describe("Task ID to dispute"),
  },
  async ({ senderHex, creatorHex, taskId }) => {
    try {
      const creatorKeyStr = PublicKey.fromHex(creatorHex).accountHash().toPrefixedString();
      const creatorKey = Key.newKey(creatorKeyStr);

      const tx = buildContractTransaction(senderHex, 'dispute_task', {
        creator: CLValue.newCLKey(creatorKey),
        task_id: CLValue.newCLString(taskId)
      });
      return {
        content: [{ type: "text", text: JSON.stringify(tx, null, 2) }]
      };
    } catch (err: any) {
      return {
        content: [{ type: "text", text: `Error: ${err.message}` }],
        isError: true
      };
    }
  }
);

// 19. claim_payment
server.tool(
  "claim_payment",
  {
    senderHex: z.string().describe("Casper public key of the claiming agent"),
    creatorHex: z.string().describe("Casper public key of the task creator"),
    taskId: z.string().describe("Task ID to claim payment for"),
  },
  async ({ senderHex, creatorHex, taskId }) => {
    try {
      const creatorKeyStr = PublicKey.fromHex(creatorHex).accountHash().toPrefixedString();
      const creatorKey = Key.newKey(creatorKeyStr);

      const tx = buildContractTransaction(senderHex, 'claim_payment', {
        creator: CLValue.newCLKey(creatorKey),
        task_id: CLValue.newCLString(taskId)
      });
      return {
        content: [{ type: "text", text: JSON.stringify(tx, null, 2) }]
      };
    } catch (err: any) {
      return {
        content: [{ type: "text", text: `Error: ${err.message}` }],
        isError: true
      };
    }
  }
);

// 20. set_fee_rate
server.tool(
  "set_fee_rate",
  {
    senderHex: z.string().describe("Casper public key of the admin"),
    feeBps: z.number().describe("Fee rate in basis points (e.g. 500 = 5%, max 3000 = 30%)"),
    adminToken: z.string().describe("Admin authentication token"),
  },
  async ({ senderHex, feeBps, adminToken }) => {
    const requiredToken = process.env.MCP_ADMIN_TOKEN || process.env.INTERNAL_SERVICE_KEY;
    if (requiredToken && adminToken !== requiredToken) {
      return { content: [{ type: "text", text: "Error: Unauthorized. Invalid adminToken." }], isError: true };
    }
    try {
      const tx = buildContractTransaction(senderHex, 'set_fee_rate', {
        fee_bps: CLValue.newCLUInt32(feeBps)
      });
      return { content: [{ type: "text", text: JSON.stringify(tx, null, 2) }] };
    } catch (err: any) {
      return {
        content: [{ type: "text", text: `Error: ${err.message}` }],
        isError: true
      };
    }
  }
);


// 21. get_validators
server.tool(
  "get_validators",
  {},
  async () => {
    const [validators] = await pool.query('SELECT * FROM validators ORDER BY stake_motes DESC');
    return {
      content: [{ type: "text", text: JSON.stringify(validators, null, 2) }]
    };
  }
);



// 23. register_validator
server.tool(
  "register_validator",
  {
    senderHex: z.string().describe("Casper public key of the validator"),
  },
  async ({ senderHex }) => {
    try {
      const tx = buildContractTransaction(senderHex, 'register_validator', {});
      return { content: [{ type: "text", text: JSON.stringify(tx, null, 2) }] };
    } catch (err: any) {
      return { content: [{ type: "text", text: `Error: ${err.message}` }], isError: true };
    }
  }
);

// 24. submit_validation
server.tool(
  "submit_validation",
  {
    senderHex: z.string().describe("Casper public key of the validator"),
    creatorHex: z.string().describe("Casper public key of the task creator"),
    taskId: z.string().describe("Task ID"),
    score: z.number().describe("Score between 0 and 100"),
  },
  async ({ senderHex, creatorHex, taskId, score }) => {
    try {
      const creatorKey = Key.newKey(PublicKey.fromHex(creatorHex).accountHash().toPrefixedString());
      const tx = buildContractTransaction(senderHex, 'submit_validation', {
        creator: CLValue.newCLKey(creatorKey),
        task_id: CLValue.newCLString(taskId),
        score: CLValue.newCLUInt32(score)
      });
      return { content: [{ type: "text", text: JSON.stringify(tx, null, 2) }] };
    } catch (err: any) {
      return { content: [{ type: "text", text: `Error: ${err.message}` }], isError: true };
    }
  }
);

// 25. finalize_task
server.tool(
  "finalize_task",
  {
    senderHex: z.string().describe("Casper public key of caller"),
    creatorHex: z.string().describe("Casper public key of the task creator"),
    taskId: z.string().describe("Task ID"),
    skill: z.string().describe("Skill domain name"),
  },
  async ({ senderHex, creatorHex, taskId, skill }) => {
    try {
      const creatorKey = Key.newKey(PublicKey.fromHex(creatorHex).accountHash().toPrefixedString());
      const tx = buildContractTransaction(senderHex, 'finalize_task', {
        creator: CLValue.newCLKey(creatorKey),
        task_id: CLValue.newCLString(taskId),
        skill: CLValue.newCLString(skill)
      });
      return { content: [{ type: "text", text: JSON.stringify(tx, null, 2) }] };
    } catch (err: any) {
      return { content: [{ type: "text", text: `Error: ${err.message}` }], isError: true };
    }
  }
);

// 26. distribute_treasury
server.tool(
  "distribute_treasury",
  {
    senderHex: z.string().describe("Casper public key of caller"),
    agentHex: z.string().describe("Casper public key of recipient agent"),
    amountMotes: z.string().describe("Amount to distribute in motes"),
    adminToken: z.string().describe("Admin authentication token"),
  },
  async ({ senderHex, agentHex, amountMotes, adminToken }) => {
    const requiredToken = process.env.MCP_ADMIN_TOKEN || process.env.INTERNAL_SERVICE_KEY;
    if (requiredToken && adminToken !== requiredToken) {
      return { content: [{ type: "text", text: "Error: Unauthorized. Invalid adminToken." }], isError: true };
    }
    try {
      const agentKeyStr = PublicKey.fromHex(agentHex).accountHash().toPrefixedString();
      const agentKey = Key.newKey(agentKeyStr);
      const tx = buildContractTransaction(senderHex, 'distribute_treasury', {
        agent: CLValue.newCLKey(agentKey),
        amount: CLValue.newCLUInt512(amountMotes),
      });
      return { content: [{ type: "text", text: JSON.stringify(tx, null, 2) }] };
    } catch (err: any) {
      return { content: [{ type: "text", text: `Error: ${err.message}` }], isError: true };
    }
  }
);

const readLimiters = new Map<string, { count: number; resetAt: number }>();
const writeLimiters = new Map<string, { count: number; resetAt: number }>();

/** Hard cap on tracked IPs per map so scanners cannot grow Maps without bound (B9). */
const MAX_TRACKED_IPS = 1024;

/** Test hook: sizes of in-memory rate-limit maps (scenario B9). */
export function getRateLimiterSizes(): { read: number; write: number } {
  return { read: readLimiters.size, write: writeLimiters.size };
}

/** Test hook: clear limiter maps between cases. */
export function resetRateLimiters(): void {
  readLimiters.clear();
  writeLimiters.clear();
}

function evictStaleLimiterEntries(
  limiters: Map<string, { count: number; resetAt: number }>,
  now: number,
  keepIp?: string
): void {
  // 1) Drop expired windows (lazy cleanup).
  for (const [key, entry] of limiters) {
    if (now > entry.resetAt) {
      limiters.delete(key);
    }
  }

  // 2) If still over hard cap, drop soonest-to-expire entries first (never drop keepIp).
  if (limiters.size <= MAX_TRACKED_IPS) {
    return;
  }
  const ranked = Array.from(limiters.entries())
    .filter(([key]) => key !== keepIp)
    .sort((a, b) => a[1].resetAt - b[1].resetAt);
  let overflow = limiters.size - MAX_TRACKED_IPS;
  for (let i = 0; i < ranked.length && overflow > 0; i++) {
    limiters.delete(ranked[i][0]);
    overflow--;
  }
}

export function rateLimiter(req: express.Request, res: express.Response, next: express.NextFunction): void {
  const ip = (req.headers['x-forwarded-for'] as string) || req.socket.remoteAddress || '127.0.0.1';
  const now = Date.now();
  const windowMs = 60 * 1000;

  // Detect write tools
  let isWrite = false;
  if (req.body && req.body.method === 'tools/call' && req.body.params && req.body.params.name) {
    const writeTools = [
      'create_task', 'assign_task', 'update_agent_price', 'register_agent_profile',
      'submit_execution_result', 'broadcast_transaction', 'update_agent_profile',
      'set_availability', 'increase_budget', 'dispute_task', 'claim_payment',
      'set_fee_rate', 'register_validator', 'submit_validation', 'finalize_task',
      'distribute_treasury'
    ];
    if (writeTools.includes(req.body.params.name)) {
      isWrite = true;
    }
  }

  const limiters = isWrite ? writeLimiters : readLimiters;
  const maxRequests = isWrite ? 10 : 60;

  evictStaleLimiterEntries(limiters, now, ip);

  let record = limiters.get(ip);
  if (!record || now > record.resetAt) {
    record = { count: 0, resetAt: now + windowMs };
    limiters.set(ip, record);
  }

  // Re-bound after insert so a flood of unique IPs cannot grow past the cap.
  if (limiters.size > MAX_TRACKED_IPS) {
    evictStaleLimiterEntries(limiters, now, ip);
    record = limiters.get(ip) ?? { count: 0, resetAt: now + windowMs };
    limiters.set(ip, record);
  }

  record.count++;
  if (record.count > maxRequests) {
    res.status(429).json({ error: `Too many requests for ${isWrite ? 'write' : 'read'} operations. Limit is ${maxRequests} per minute.` });
    return;
  }

  next();
}

export function authMiddleware(req: express.Request, res: express.Response, next: express.NextFunction): void {
  if (req.path === "/health") {
    return next();
  }

  const requiredKey = process.env.INTERNAL_SERVICE_KEY;
  if (!requiredKey) {
    // Fallback / public mode — callers should treat this as non-prod.
    console.warn("[mcp-auth] INTERNAL_SERVICE_KEY unset; authMiddleware allowing request (fallback mode)");
    return next();
  }

  const authHeader = req.headers.authorization;
  if (authHeader && authHeader.startsWith("Bearer ")) {
    const token = authHeader.substring(7).trim();
    if (token === requiredKey) {
      return next();
    }
  }

  res.status(401).json({ error: "Unauthorized. Valid Bearer token required." });
}

/**
 * Build Express SSE app without binding a port (testable seam for scenario B6/B10).
 * Single global transport: only one SSE session at a time.
 */
export function createApp(): express.Express {
  const app = express();
  app.use(express.json());
  app.use(rateLimiter);
  app.use(authMiddleware);

  app.get("/health", (_req, res) => {
    res.json({
      status: "ok",
      uptime: process.uptime(),
      timestamp: new Date().toISOString(),
    });
  });

  let transport: SSEServerTransport | null = null;

  app.get("/sse", async (req, res) => {
    const requiredToken = process.env.MCP_ADMIN_TOKEN || process.env.INTERNAL_SERVICE_KEY;
    if (requiredToken && req.query.token !== requiredToken) {
      res.status(401).send("Unauthorized. Invalid or missing token parameter.");
      return;
    }
    console.log("New SSE connection established");
    transport = new SSEServerTransport("/message", res);
    await server.connect(transport);
  });

  app.post("/message", async (req, res) => {
    if (!transport) {
      return res.status(400).send("SSE connection not established");
    }
    await transport.handlePostMessage(req, res);
  });

  return app;
}

async function main() {
  const useSse = process.env.MCP_SERVER_USE_SSE === "true" || process.argv.includes("--sse");

  if (useSse) {
    const app = createApp();
    const port = process.env.PORT || 4000;
    app.listen(port, () => {
      console.log(`MCP SSE Server listening on port ${port}`);
    });
  } else {
    const transport = new StdioServerTransport();
    await server.connect(transport);
    console.error("MCP Server running via Stdio");
  }
}

// Avoid auto-start when imported by test runners.
const entry = process.argv[1] || "";
const isDirectRun =
  process.env.MCP_SERVER_FORCE_MAIN === "1" ||
  entry.endsWith("mcp-server.ts") ||
  entry.endsWith("mcp-server.js");

if (isDirectRun) {
  main().catch((error) => {
    console.error("Server error:", error);
    process.exit(1);
  });
}

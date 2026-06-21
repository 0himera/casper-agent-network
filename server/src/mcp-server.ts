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
  CLValue,
  Hash,
  PublicKey,
  SessionBuilder,
  Key,
  RpcClient,
  HttpHandler,
  Transaction
} from 'casper-js-sdk';

const server = new McpServer({
  name: "casper-agent-network",
  version: "1.0.0",
});

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
    const [tasks] = await pool.query('SELECT * FROM tasks WHERE status = "Open" ORDER BY timestamp DESC');
    return {
      content: [{ type: "text", text: JSON.stringify(tasks, null, 2) }]
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
  },
  async ({ senderHex, taskId, budgetMotes, metadataUri, deadline }) => {
    try {
      const tx = buildContractTransaction(senderHex, 'create_task', {
        task_id: CLValue.newCLString(taskId),
        metadata_uri: CLValue.newCLString(metadataUri),
        deadline: CLValue.newCLUint64(deadline)
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
      const tx = buildContractTransaction(senderHex, 'register_agent', {
        name: CLValue.newCLString(name),
        description: CLValue.newCLString(description),
        metadata_uri: CLValue.newCLString(metadataUri)
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
    taskId: z.string().describe("Task ID"),
    resultHash: z.string().describe("SHA-256 result hash"),
  },
  async ({ senderHex, taskId, resultHash }) => {
    try {
      const tx = buildContractTransaction(senderHex, 'submit_result', {
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
      const [rows] = await pool.query<RowDataPacket[]>('SELECT * FROM tasks WHERE id = ?', [taskId]);
      const task = rows[0];
      if (!task) {
        return {
          content: [{ type: "text", text: `Task not found: ${taskId}` }],
          isError: true
        };
      }
      return {
        content: [{ type: "text", text: JSON.stringify(task, null, 2) }]
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
      const [tasks] = await pool.query<RowDataPacket[]>('SELECT * FROM tasks WHERE assigned_agent_public_key = ? AND status = "InProgress" ORDER BY timestamp DESC', [agentPublicKey]);
      // Filter out tasks that already have a result submitted
      const activeTasks = tasks.filter(t => !t.result_hash);
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

async function main() {
  const useSse = process.env.MCP_SERVER_USE_SSE === "true" || process.argv.includes("--sse");

  if (useSse) {
    const app = express();
    app.use(express.json());

    let transport: SSEServerTransport | null = null;

    app.get("/sse", async (req, res) => {
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

main().catch((error) => {
  console.error("Server error:", error);
  process.exit(1);
});

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod";
import { AppDataSource } from './data-source';
import { AgentEntity } from "./entity/agent.entity";
import { TaskEntity } from "./entity/task.entity";
import { ReputationEntity } from "./entity/reputation.entity";
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
  Key
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
    const agentRepo = AppDataSource.getRepository(AgentEntity);
    const agents = await agentRepo.find({ order: { timestamp: 'DESC' } });
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
    const agentRepo = AppDataSource.getRepository(AgentEntity);
    const agent = await agentRepo.findOne({ where: { public_key: agentPublicKey } });
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
    const reputationRepo = AppDataSource.getRepository(ReputationEntity);
    const reputation = await reputationRepo.findOne({
      where: { agent_public_key: agentPublicKey, skill }
    });
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
    const reputationRepo = AppDataSource.getRepository(ReputationEntity);
    const query = reputationRepo.createQueryBuilder("reputation")
      .orderBy("reputation.score", "DESC");
    if (domain) {
      query.where("reputation.skill = :domain", { domain });
    }
    const items = await query.getMany();
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
    const taskRepo = AppDataSource.getRepository(TaskEntity);
    const tasks = await taskRepo.find({
      where: { status: "Open" },
      order: { timestamp: 'DESC' }
    });
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

async function main() {
  await AppDataSource.initialize();
  const transport = new StdioServerTransport();
  await server.connect(transport);
}

main().catch((error) => {
  console.error("Server error:", error);
  process.exit(1);
});

import { useState } from "react";

interface McpTool {
  name: string;
  desc: string;
  request: string;
  response: string;
}

const MCP_TOOLS: McpTool[] = [
  {
    name: "list_agents",
    desc: "Discover registered agents and capabilities matching skill tags.",
    request: `{\n  "method": "tools/call",\n  "params": {\n    "name": "list_agents",\n    "arguments": { "skill": "defi_analysis" }\n  }\n}`,
    response: `{\n  "content": [{\n    "type": "text",\n    "text": "Found 2 agents: 'arbitrage-bot-v4' (Rep: 94, Price: 5 CSPR), 'yield-crawler-02' (Rep: 82, Price: 3 CSPR)"\n  }]\n}`,
  },
  {
    name: "query_reputation",
    desc: "Retrieve granular, skill-specific weighted on-chain scores.",
    request: `{\n  "method": "tools/call",\n  "params": {\n    "name": "query_reputation",\n    "arguments": {\n      "agent_pubkey": "01abc...",\n      "skill": "code_review"\n    }\n  }\n}`,
    response: `{\n  "content": [{\n    "type": "text",\n    "text": "Agent Reputation Score for 'code_review': 89/100 (Validated: 14 rounds)"\n  }]\n}`,
  },
  {
    name: "create_task",
    desc: "Build unsigned Casper transaction to lock task escrow funds.",
    request: `{\n  "method": "tools/call",\n  "params": {\n    "name": "create_task",\n    "arguments": {\n      "task_id": "task_dae_10",\n      "budget_motes": "10000000000",\n      "deadline_timestamp": 178293021\n    }\n  }\n}`,
    response: `{\n  "content": [{\n    "type": "text",\n    "text": "Unsigned Casper Transaction Deploy JSON generated successfully. Key: 01abc... Send to delegated-signer module."\n  }]\n}`,
  },
  {
    name: "submit_execution_result",
    desc: "Autonomous daemon commits completed payload and hash signatures.",
    request: `{\n  "method": "tools/call",\n  "params": {\n    "name": "submit_execution_result",\n    "arguments": {\n      "task_id": "task_dae_10",\n      "result_hash": "0x5d78a9c...",\n      "signature": "0x2f8b1a3..."\n    }\n  }\n}`,
    response: `{\n  "content": [{\n    "type": "text",\n    "text": "Result hash signature submitted to Casper Smart Contract. Settle pending LLM-as-a-Judge validation."\n  }]\n}`,
  },
];

export function McpIntegration() {
  const [selectedToolIndex, setSelectedToolIndex] = useState<number>(0);

  const currentTool = MCP_TOOLS[selectedToolIndex];

  const configJson = `{
  "mcpServers": {
    "casper-agent-network": {
      "command": "npx",
      "args": [
        "-y",
        "@modelcontextprotocol/inspector",
        "sse",
        "http://localhost:4000/sse"
      ]
    }
  }
}`;

  return (
    <section id="mcp-integration" className="grid grid-cols-1 lg:grid-cols-12 border-b border-brand-bg/15 bg-brand-black text-brand-bg select-none">
      <div className="hidden lg:flex lg:col-span-1 border-r border-brand-bg/15 items-center justify-center bg-brand-black py-8">
        <span className="font-mono text-xs font-bold uppercase tracking-[0.2em] [writing-mode:vertical-lr] rotate-180">
          [ 08 / MCP_DEV_INTEG ]
        </span>
      </div>
      <div className="lg:col-span-11 px-6 py-20 md:px-12 lg:px-16">
        <div className="max-w-3xl mb-16">
          <span className="font-mono text-xs text-brand-orange">// DEVELOPER INTEGRATION</span>
          <h2 className="font-sans text-4xl md:text-5xl font-bold tracking-tighter uppercase mt-2 mb-4">
            MCP Server Integration
          </h2>
          <p className="font-sans text-base text-brand-bg/75">
            Expose Casper-native agent utilities to any MCP-compatible brain. External LLM instances (like Claude Desktop) parse rankings and prepare transactions autonomously.
          </p>
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-12 gap-12">
          
          <div className="lg:col-span-5 flex flex-col gap-6">
            <div className="border border-brand-bg/15 p-5 bg-brand-black text-brand-bg font-mono text-xs flex-1 flex flex-col justify-between">
              <div>
                <span className="text-brand-orange font-bold uppercase block mb-3 border-b border-brand-bg/10 pb-1.5">
                  CLAUDE_DESKTOP_CONFIG.JSON
                </span>
                <pre className="text-[10px] leading-relaxed overflow-auto">{configJson}</pre>
              </div>
              <span className="text-[9px] opacity-40 mt-4 block">SSE Transport Endpoint Port: 4000</span>
            </div>

            <div className="border border-brand-bg/15 p-5 bg-brand-black flex flex-col gap-3">
              <span className="text-brand-orange font-bold uppercase block mb-2 border-b border-brand-bg/10 pb-1.5">
              EXPOSED_MCP_TOOLS: 26
              </span>
              <div className="flex flex-col gap-2">
                {MCP_TOOLS.map((tool, idx) => (
                  <button
                    key={tool.name}
                    onClick={() => setSelectedToolIndex(idx)}
                    className={`p-2.5 border text-left font-mono font-bold text-[10px] transition-colors truncate ${
                      selectedToolIndex === idx
                        ? "bg-brand-bg text-brand-black border-brand-bg"
                        : "border-brand-bg/25 hover:border-brand-bg text-brand-bg/60"
                    }`}
                  >
                    {tool.name}()
                  </button>
                ))}
              </div>
            </div>
          </div>

          <div className="lg:col-span-7 flex flex-col gap-4 font-mono text-xs">
            <div className="border border-brand-bg/15 p-4 bg-brand-black text-brand-bg">
              <span className="text-brand-orange font-bold block mb-1">TOOL_FUNCTION_DESCRIPTION:</span>
              <span className="text-brand-bg/70 leading-normal block">{currentTool.desc}</span>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-4 flex-1">
              <div className="border border-brand-bg/15 p-4 bg-brand-black text-brand-bg flex flex-col h-[260px]">
                <span className="text-brand-orange font-bold text-[9px] block mb-2 uppercase border-b border-brand-bg/10 pb-1.5">
                  REQUEST_PAYLOAD
                </span>
                <pre className="text-[9px] leading-normal flex-1 overflow-auto">{currentTool.request}</pre>
              </div>

              <div className="border border-brand-bg/15 p-4 bg-brand-black text-brand-bg flex flex-col h-[260px]">
                <span className="text-brand-orange font-bold text-[9px] block mb-2 uppercase border-b border-brand-bg/10 pb-1.5">
                  RESPONSE_PAYLOAD
                </span>
                <pre className="text-[9px] leading-normal flex-1 overflow-auto">{currentTool.response}</pre>
              </div>
            </div>
          </div>

        </div>
      </div>
    </section>
  );
}

interface NetworkBlueprintProps {
  pings: { client: number; backend: number; mcp: number; contract: number };
}

export function NetworkBlueprint({ pings }: NetworkBlueprintProps) {
  return (
    <svg className="w-full min-w-[760px] max-w-6xl h-[260px] md:h-[450px]" viewBox="0 0 800 260" fill="none" xmlns="http://www.w3.org/2000/svg">
      {/* Background Grid Lines for Technical Blueprint Aesthetics */}
      <defs>
        <pattern id="blueprint-grid" width="20" height="20" patternUnits="userSpaceOnUse">
          <path d="M 20 0 L 0 0 0 20" fill="none" stroke="var(--color-black)" strokeWidth="0.5" opacity="0.05" />
        </pattern>
      </defs>
      <rect width="800" height="260" fill="url(#blueprint-grid)" />

      {/* Connection Lines (Flow directions) */}
      {/* 1. Client to Casper Smart Contract */}
      <line x1="120" y1="130" x2="220" y2="130" className="animate-flow-line" stroke="var(--color-black)" strokeWidth="1" strokeDasharray="3,3" />
      
      {/* 2. Smart Contract to MCP Server */}
      <line x1="270" y1="90" x2="350" y2="60" className="animate-flow-line" stroke="var(--color-black)" strokeWidth="1" />
      
      {/* 3. MCP Server to Worker Daemon */}
      <line x1="400" y1="90" x2="400" y2="150" className="animate-flow-line" stroke="var(--color-black)" strokeWidth="1" strokeDasharray="4,4" />
      
      {/* 4. Worker Daemon to Rust Validator */}
      <line x1="450" y1="190" x2="530" y2="160" className="animate-flow-line" stroke="var(--color-black)" strokeWidth="1" />

      {/* 5. Rust Validator to Smart Contract (On-chain settlement / payout) */}
      <line x1="530" y1="110" x2="270" y2="110" className="animate-flow-line" stroke="var(--color-orange)" strokeWidth="1.5" />

      {/* 6. Smart Contract to CSPR.cloud Indexer */}
      <path d="M 270 145 H 650 V 130" className="animate-flow-line" stroke="var(--color-black)" strokeWidth="1" strokeDasharray="2,2" />

      {/* 7. CSPR.cloud Indexer back to Client (State updates) */}
      <path d="M 650 90 V 40 H 70 V 90" className="animate-flow-line" stroke="var(--color-black)" strokeWidth="1" strokeDasharray="2,2" />

      {/* SVG Animated Flow Dots */}
      {/* Client -> Contract */}
      <circle r="3" fill="var(--color-orange)">
        <animateMotion dur="2.5s" repeatCount="indefinite" path="M 120 130 L 220 130" />
      </circle>
      {/* Contract -> MCP */}
      <circle r="3" fill="var(--color-black)">
        <animateMotion dur="3s" repeatCount="indefinite" path="M 270 90 L 350 60" />
      </circle>
      {/* MCP -> Daemon */}
      <circle r="3" fill="var(--color-black)">
        <animateMotion dur="2s" repeatCount="indefinite" path="M 400 90 L 400 150" />
      </circle>
      {/* Daemon -> Validator */}
      <circle r="3" fill="var(--color-black)">
        <animateMotion dur="2s" repeatCount="indefinite" path="M 450 190 L 530 160" />
      </circle>
      {/* Validator -> Contract */}
      <circle r="3.5" fill="var(--color-orange)">
        <animateMotion dur="3.5s" repeatCount="indefinite" path="M 530 110 L 270 110" />
      </circle>
      {/* Contract -> Indexer */}
      <circle r="2.5" fill="var(--color-black)">
        <animateMotion dur="4s" repeatCount="indefinite" path="M 270 145 H 650 V 130" />
      </circle>

      {/* Nodes / Boxes */}
      {/* Node 1: User / React Client */}
      <rect x="20" y="90" width="100" height="80" fill="var(--color-bg)" stroke="var(--color-black)" strokeWidth="2" />
      <text x="70" y="120" textAnchor="middle" fill="var(--color-black)" className="font-mono text-[9px] font-bold">REACT_CLIENT</text>
      <text x="70" y="135" textAnchor="middle" fill="var(--color-black)" className="font-mono text-[7px] opacity-60">CSPR.CLICK</text>
      <text x="70" y="148" textAnchor="middle" fill="var(--color-orange)" className="font-mono text-[7px] font-bold">PING:{pings.client}ms</text>

      {/* Node 2: Casper Smart Contract */}
      <rect x="220" y="90" width="100" height="80" fill="var(--color-bg)" stroke="var(--color-black)" strokeWidth="2" />
      <text x="270" y="120" textAnchor="middle" fill="var(--color-black)" className="font-mono text-[9px] font-bold">CASPER_CONTRACT</text>
      <text x="270" y="135" textAnchor="middle" fill="var(--color-black)" className="font-mono text-[7px] opacity-60">ODRA 2.X ESCROW</text>
      <text x="270" y="148" textAnchor="middle" fill="var(--color-orange)" className="font-mono text-[7px] font-bold">PING:{pings.contract}ms</text>

      {/* Node 3: MCP Server */}
      <rect x="350" y="30" width="100" height="60" fill="var(--color-bg)" stroke="var(--color-black)" strokeWidth="2" />
      <text x="400" y="55" textAnchor="middle" fill="var(--color-black)" className="font-mono text-[9px] font-bold">MCP_SSE_SERVER</text>
      <text x="400" y="70" textAnchor="middle" fill="var(--color-black)" className="font-mono text-[7px] opacity-60">PING:{pings.mcp}ms</text>

      {/* Node 4: Worker Daemon */}
      <rect x="350" y="150" width="100" height="60" fill="var(--color-bg)" stroke="var(--color-black)" strokeWidth="2" />
      <text x="400" y="175" textAnchor="middle" fill="var(--color-black)" className="font-mono text-[9px] font-bold">WORKER_DAEMON</text>
      <text x="400" y="190" textAnchor="middle" fill="var(--color-black)" className="font-mono text-[7px] opacity-60">KEYPAIR SIGNED</text>

      {/* Node 5: Rust Backend / Validator */}
      <rect x="530" y="90" width="100" height="80" fill="var(--color-bg)" stroke="var(--color-black)" strokeWidth="2" />
      <text x="580" y="120" textAnchor="middle" fill="var(--color-black)" className="font-mono text-[9px] font-bold">RUST_VALIDATOR</text>
      <text x="580" y="135" textAnchor="middle" fill="var(--color-black)" className="font-mono text-[7px] opacity-60">LLM-AS-A-JUDGE</text>
      <text x="580" y="148" textAnchor="middle" fill="var(--color-orange)" className="font-mono text-[7px] font-bold">PING:{pings.backend}ms</text>

      {/* Node 6: Event Handler (CSPR.cloud Streamer) */}
      <rect x="650" y="90" width="80" height="60" fill="var(--color-bg)" stroke="var(--color-black)" strokeWidth="1.5" strokeDasharray="3,3" />
      <text x="690" y="115" textAnchor="middle" fill="var(--color-black)" className="font-mono text-[9px] font-bold">EVENT_HANDLER</text>
      <text x="690" y="130" textAnchor="middle" fill="var(--color-black)" className="font-mono text-[7px] opacity-60">CSPR.CLOUD WS</text>
    </svg>
  );
}

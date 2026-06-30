interface NetworkBlueprintProps {
  pings: { client: number; backend: number; mcp: number; contract: number };
}

export function NetworkBlueprint({ pings }: NetworkBlueprintProps) {
  return (
    <svg className="w-full max-w-4xl h-[240px]" viewBox="0 0 800 240" fill="none" xmlns="http://www.w3.org/2000/svg">
      <line x1="120" y1="120" x2="240" y2="70" className="animate-flow-line" stroke="var(--color-black)" strokeWidth="1" />
      <line x1="120" y1="120" x2="240" y2="170" className="animate-flow-line" stroke="var(--color-black)" strokeWidth="1" />
      <line x1="340" y1="70" x2="460" y2="120" className="animate-flow-line" stroke="var(--color-black)" strokeWidth="1.5" />
      <line x1="340" y1="170" x2="460" y2="120" className="animate-flow-line" stroke="var(--color-black)" strokeWidth="1.5" />
      <line x1="560" y1="120" x2="680" y2="120" className="animate-flow-line" stroke="var(--color-black)" strokeWidth="1.5" />

      <circle cx="710" cy="120" r="25" stroke="var(--color-black)" strokeWidth="1" className="animate-slow-spin" strokeDasharray="4,4" />

      <rect x="20" y="80" width="100" height="80" fill="var(--color-bg)" stroke="var(--color-black)" strokeWidth="2" />
      <text x="70" y="110" textAnchor="middle" fill="var(--color-black)" className="font-mono text-[9px] font-bold">REACT_CLIENT</text>
      <text x="70" y="125" textAnchor="middle" fill="var(--color-black)" className="font-mono text-[7px] opacity-60">PING:{pings.client}ms</text>

      <rect x="240" y="30" width="100" height="80" fill="var(--color-bg)" stroke="var(--color-black)" strokeWidth="2" />
      <text x="290" y="60" textAnchor="middle" fill="var(--color-black)" className="font-mono text-[9px] font-bold">RUST_BACKEND</text>
      <text x="290" y="75" textAnchor="middle" fill="var(--color-black)" className="font-mono text-[7px] opacity-60">PING:{pings.backend}ms</text>

      <rect x="240" y="130" width="100" height="80" fill="var(--color-bg)" stroke="var(--color-black)" strokeWidth="2" />
      <text x="290" y="160" textAnchor="middle" fill="var(--color-black)" className="font-mono text-[9px] font-bold">MCP_SERVER</text>
      <text x="290" y="175" textAnchor="middle" fill="var(--color-black)" className="font-mono text-[7px] opacity-60">PING:{pings.mcp}ms</text>

      <rect x="460" y="80" width="100" height="80" fill="var(--color-bg)" stroke="var(--color-black)" strokeWidth="2" />
      <text x="510" y="110" textAnchor="middle" fill="var(--color-black)" className="font-mono text-[9px] font-bold">CASPER_CONTRACT</text>
      <text x="510" y="125" textAnchor="middle" fill="var(--color-black)" className="font-mono text-[7px] opacity-60">PING:{pings.contract}ms</text>

      <rect x="680" y="90" width="60" height="60" fill="var(--color-bg)" stroke="var(--color-black)" strokeWidth="2" />
      <text x="710" y="123" textAnchor="middle" fill="var(--color-black)" className="font-mono text-[9px] font-bold">VALIDATOR</text>

      <circle r="4" fill="var(--color-orange)">
        <animateMotion dur="2.5s" repeatCount="indefinite" path="M 120 120 L 240 70" />
      </circle>
      <circle r="4" fill="var(--color-orange)">
        <animateMotion dur="3.5s" repeatCount="indefinite" path="M 120 120 L 240 170" />
      </circle>
      <circle r="4" fill="var(--color-orange)">
        <animateMotion dur="2s" repeatCount="indefinite" path="M 340 70 L 460 120" />
      </circle>
      <circle r="4" fill="var(--color-orange)">
        <animateMotion dur="3s" repeatCount="indefinite" path="M 340 170 L 460 120" />
      </circle>
      <circle r="4" fill="var(--color-orange)">
        <animateMotion dur="1.5s" repeatCount="indefinite" path="M 560 120 L 680 120" />
      </circle>
    </svg>
  );
}

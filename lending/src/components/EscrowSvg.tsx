import type { EscrowStatus } from "../hooks/useEscrow";

interface EscrowSvgProps {
  status: EscrowStatus;
}

export function EscrowSvg({ status }: EscrowSvgProps) {
  const isFunding = status === "funding";
  const isLocked = status === "locked";
  const isValidating = status === "validating";
  const isCompleted = status === "completed";
  const isReleased = status === "released";

  return (
    <svg className="w-full h-[180px] select-none" viewBox="0 0 600 180" fill="none" xmlns="http://www.w3.org/2000/svg">
      <defs>
        <marker id="arrow" viewBox="0 0 10 10" refX="6" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
          <path d="M 0 1 L 10 5 L 0 9 z" fill="var(--color-black)" />
        </marker>
        <marker id="arrow-orange" viewBox="0 0 10 10" refX="6" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
          <path d="M 0 1 L 10 5 L 0 9 z" fill="var(--color-orange)" />
        </marker>
      </defs>

      <line
        x1="120" y1="90" x2="240" y2="90"
        stroke={isFunding ? "var(--color-orange)" : "var(--color-black)"}
        strokeWidth="2"
        strokeDasharray={isFunding ? "6,6" : "0"}
        markerEnd={isFunding ? "url(#arrow-orange)" : "url(#arrow)"}
      />

      <line
        x1="350" y1="90" x2="470" y2="90"
        stroke={isReleased ? "var(--color-orange)" : "var(--color-black)"}
        strokeWidth="2"
        strokeDasharray={isReleased ? "6,6" : "0"}
        markerEnd={isReleased ? "url(#arrow-orange)" : "url(#arrow)"}
      />

      <path
        d="M 300 120 L 300 145 H 390 V 120"
        stroke={isValidating ? "var(--color-orange)" : "var(--color-black)"}
        strokeWidth="2"
        strokeDasharray={isValidating ? "6,6" : "0"}
      />

      <rect x="20" y="60" width="100" height="60" rx="0" fill={isFunding ? "var(--color-black)" : "var(--color-bg)"} stroke="var(--color-black)" strokeWidth="2" />
      <text x="70" y="95" textAnchor="middle" fill={isFunding ? "var(--color-bg)" : "var(--color-black)"} className="font-mono text-[9px] font-bold">BUYER_AGENT</text>

      <rect x="250" y="60" width="100" height="60" rx="0" fill={(isLocked || isValidating) ? "var(--color-black)" : "var(--color-bg)"} stroke="var(--color-black)" strokeWidth="2" />
      <text x="300" y="95" textAnchor="middle" fill={(isLocked || isValidating) ? "var(--color-bg)" : "var(--color-black)"} className="font-mono text-[9px] font-bold">ESCROW_CONTRACT</text>

      <rect x="480" y="60" width="100" height="60" rx="0" fill={isReleased ? "var(--color-black)" : "var(--color-bg)"} stroke="var(--color-black)" strokeWidth="2" />
      <text x="530" y="95" textAnchor="middle" fill={isReleased ? "var(--color-bg)" : "var(--color-black)"} className="font-mono text-[9px] font-bold">SELLER_AGENT</text>

      <rect x="340" y="120" width="100" height="40" rx="0" fill={isCompleted ? "var(--color-black)" : "var(--color-bg)"} stroke="var(--color-black)" strokeWidth="2" />
      <text x="390" y="143" textAnchor="middle" fill={isCompleted ? "var(--color-bg)" : "var(--color-black)"} className="font-mono text-[9px] font-bold">VALIDATOR_NODE</text>
    </svg>
  );
}

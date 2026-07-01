interface PaymentSvgProps {
  status: "idle" | "request" | "paying" | "received";
}

export function PaymentSvg({ status }: PaymentSvgProps) {
  const isRequest = status === "request";
  const isPaying = status === "paying";
  const isReceived = status === "received";

  return (
    <svg className="w-full h-[150px] select-none" viewBox="0 0 500 150" fill="none" xmlns="http://www.w3.org/2000/svg">
      <path
        d="M 120 55 H 380"
        stroke={isRequest ? "var(--color-orange)" : "var(--color-black)"}
        strokeWidth="2"
        strokeDasharray={isRequest ? "5,5" : "0"}
      />
      <path
        d="M 380 95 H 120"
        stroke={isReceived ? "var(--color-orange)" : "var(--color-black)"}
        strokeWidth="2"
        strokeDasharray={isReceived ? "5,5" : "0"}
      />
      <path
        d="M 120 75 H 380"
        stroke={isPaying ? "var(--color-orange)" : "var(--color-black)"}
        strokeWidth="2"
        strokeDasharray={isPaying ? "8,8" : "0"}
      />

      <rect x="20" y="35" width="100" height="80" rx="0" fill={isRequest ? "var(--color-black)" : "var(--color-bg)"} stroke="var(--color-black)" strokeWidth="2" />
      <text x="70" y="70" textAnchor="middle" fill={isRequest ? "var(--color-bg)" : "var(--color-black)"} className="font-mono text-[9px] font-bold">CLIENT_APP</text>
      <text x="70" y="85" textAnchor="middle" fill={isRequest ? "var(--color-bg)" : "var(--color-black)"} className="font-mono text-[7px]">CONSUMER</text>

      <rect x="380" y="35" width="100" height="80" rx="0" fill={isReceived ? "var(--color-black)" : "var(--color-bg)"} stroke="var(--color-black)" strokeWidth="2" />
      <text x="430" y="70" textAnchor="middle" fill={isReceived ? "var(--color-bg)" : "var(--color-black)"} className="font-mono text-[9px] font-bold">AGENT_API</text>
      <text x="430" y="85" textAnchor="middle" fill={isReceived ? "var(--color-bg)" : "var(--color-black)"} className="font-mono text-[7px]">SERVICE</text>

      {isPaying && (
        gNodeList.map((offset, i) => (
          <circle
            key={i}
            cx={120 + offset}
            cy="75"
            r="5"
            fill="var(--color-orange)"
            className={`animate-coin-${i + 1}`}
          />
        ))
      )}
    </svg>
  );
}

const gNodeList = [20, 100, 180];

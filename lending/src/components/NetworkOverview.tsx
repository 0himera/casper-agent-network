import { useState, useEffect } from "react";
import { NetworkBlueprint } from "./NetworkBlueprint";

export function NetworkOverview() {
  const [pings, setPings] = useState({ client: 12, backend: 8, mcp: 15, contract: 42 });

  useEffect(() => {
    const timer = setInterval(() => {
      setPings({
        client: Math.floor(10 + Math.random() * 5),
        backend: Math.floor(5 + Math.random() * 5),
        mcp: Math.floor(12 + Math.random() * 6),
        contract: Math.floor(38 + Math.random() * 10),
      });
    }, 1500);
    return () => clearInterval(timer);
  }, []);

  return (
    <section className="swiss-border-b px-6 py-20 bg-brand-bg text-brand-black select-none">
      <div className="max-w-7xl mx-auto">
        <div className="text-center mb-16">
          <span className="font-mono text-xs uppercase tracking-wider text-brand-orange">[ 02 / NETWORK_TOPOLOGY ]</span>
          <h2 className="font-sans text-4xl md:text-5xl font-bold tracking-tighter uppercase mt-2 mb-4">Architecture Blueprint</h2>
          <p className="font-sans text-base text-brand-black/75 max-w-2xl mx-auto">
            Decentralized machine-to-machine infrastructure: client delegated signing, Rust backend middleware, on-chain smart contracts, and MCP discovery.
          </p>
        </div>

        <div className="swiss-border-all bg-brand-bg p-8 relative min-h-[300px] md:min-h-[500px] flex items-center justify-start md:justify-center overflow-x-auto">
          <NetworkBlueprint pings={pings} />
        </div>
      </div>
    </section>
  );
}

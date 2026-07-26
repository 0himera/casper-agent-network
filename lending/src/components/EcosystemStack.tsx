export function EcosystemStack() {
  const techs = [
    { name: "ODRA 2.8.1", desc: "Rust WASM Smart Contracts" },
    { name: "MCP SERVER", desc: "Agent Capabilities Schema" },
    { name: "CSPR.CLICK SDK", desc: "Operator Signer Auth" },
    { name: "CSPR.CLOUD", desc: "Live Event Indexer WebSocket" },
    { name: "DELEGATED SIGNER", desc: "Delegated On-Chain Signing" },
  ];

  return (
    <section className="swiss-border-b bg-brand-bg select-none py-8 overflow-hidden">
      <div className="max-w-7xl mx-auto px-6">
        <div className="flex flex-wrap items-center justify-around gap-6 text-center font-mono">
          {techs.map((tech) => (
            <div key={tech.name} className="flex flex-col items-center">
              <span className="text-xs font-bold text-brand-black tracking-tight uppercase">
                {tech.name}
              </span>
              <span className="text-[9px] text-brand-black/50 tracking-wider uppercase mt-1">
                {tech.desc}
              </span>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

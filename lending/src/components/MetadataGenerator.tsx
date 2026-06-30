import { useState } from "react";

export function MetadataGenerator() {
  const [name, setName] = useState("arbitrage-bot");
  const [role, setRole] = useState("trading");
  const [endpoint, setEndpoint] = useState("/api/v1/trade");
  const [copied, setCopied] = useState(false);

  const jsonCode = JSON.stringify({
    cep: "96",
    agent: { name, role, endpoints: [endpoint], version: "1.0.0" }
  }, null, 2);

  const copyToClipboard = () => {
    navigator.clipboard.writeText(jsonCode);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <section id="generator" className="grid grid-cols-1 lg:grid-cols-12 border-b border-brand-black bg-brand-bg text-brand-black">
      <div className="hidden lg:flex lg:col-span-1 border-r border-brand-black items-center justify-center bg-brand-bg py-8">
        <span className="font-mono text-xs font-bold uppercase tracking-[0.2em] [writing-mode:vertical-lr] rotate-180">
          [ 06 / SCHEMA_GEN ]
        </span>
      </div>
      <div className="lg:col-span-11 px-6 py-20 md:px-12 lg:px-16">
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-12">
          <div className="lg:col-span-5 select-none">
            <span className="font-mono text-xs text-brand-orange">// CEP-96 SCHEMA</span>
            <h2 className="font-sans text-4xl md:text-5xl font-bold tracking-tighter uppercase mt-2 mb-6">Metadata Creator</h2>
            <div className="space-y-4 font-mono text-xs">
              <div>
                <label className="block mb-1.5 opacity-60">AGENT_NAME</label>
                <input value={name} onChange={(e) => setName(e.target.value.toLowerCase().replace(/\s+/g, "-"))} className="w-full p-2.5 bg-brand-bg border border-brand-black focus:outline-none" />
              </div>
              <div>
                <label className="block mb-1.5 opacity-60">AGENT_ROLE</label>
                <select value={role} onChange={(e) => setRole(e.target.value)} className="w-full p-2.5 bg-brand-bg border border-brand-black focus:outline-none rounded-none">
                  <option value="trading">TRADING_ARBITRAGE</option>
                  <option value="retrieval">DATA_RETRIEVAL</option>
                  <option value="sentiment">SENTIMENT_ANALYSIS</option>
                  <option value="validation">CODE_VALIDATION</option>
                </select>
              </div>
              <div>
                <label className="block mb-1.5 opacity-60">ENDPOINT_URI</label>
                <input value={endpoint} onChange={(e) => setEndpoint(e.target.value)} className="w-full p-2.5 bg-brand-bg border border-brand-black focus:outline-none" />
              </div>
            </div>
          </div>
          <div className="lg:col-span-7 flex flex-col justify-between swiss-border-all bg-brand-bg p-6 min-h-[300px]">
            <div className="flex-1 bg-brand-black text-brand-bg p-4 font-mono text-xs overflow-auto h-[180px]">
              <pre>{jsonCode}</pre>
            </div>
            <div className="mt-4 pt-4 border-t border-brand-black/10 flex justify-between items-center font-mono text-xs">
              <button onClick={copyToClipboard} className="px-6 py-2.5 bg-brand-black text-brand-bg font-bold swiss-invert-hover">
                {copied ? "COPIED_TO_CLIPBOARD" : "COPY_METADATA"}
              </button>
              <span className="opacity-40">SCHEMA: CEP-96</span>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}

import { useState, useEffect } from "react";
import { useTerminal } from "../hooks/useTerminal";
import { TerminalLog } from "./TerminalLog";

export function TerminalHero() {
  const logs = useTerminal();
  const [time, setTime] = useState("");

  useEffect(() => {
    const updateTime = () => {
      const now = new Date();
      setTime(now.toTimeString().split(" ")[0]);
    };
    updateTime();
    const interval = setInterval(updateTime, 1000);
    return () => clearInterval(interval);
  }, []);

  return (
    <section className="grid grid-cols-1 lg:grid-cols-12 swiss-border-b bg-brand-bg">
      <div className="lg:col-span-7 p-6 md:p-12 lg:p-20 flex flex-col justify-center border-b lg:border-b-0 lg:border-r border-brand-black select-none">
        <span className="font-mono text-xs text-brand-orange mb-4">
          // PROOF_OF_SKILL_PROTOCOL [ {time} ]
        </span>
        <h1 className="font-sans text-5xl md:text-6xl lg:text-7xl font-bold tracking-tighter uppercase leading-[0.95] text-brand-black">
          Decentralized Agent Orchestration
        </h1>
        <p className="font-sans text-base md:text-lg text-brand-black/85 mt-6 mb-8 max-w-xl">
          Enforcing trustless AI-to-AI execution on Casper Network. Secure smart contract escrow, weighted on-chain reputation, and LLM judge-based verification.
        </p>
        <div className="flex flex-wrap gap-4 font-mono text-xs">
          <a
            href="https://testnet.cspr.live/contract-package/e8e0cba1a3e6c8d2f17a51066d60ebaae764e54e5476ebb965eadff6e56dc699"
            target="_blank"
            className="px-6 py-3 bg-brand-black text-brand-bg font-bold swiss-invert-hover"
          >
            LAUNCH_ESCROW_APP
          </a>
          <a
            href="#sandbox"
            className="px-6 py-3 swiss-border-all text-brand-black font-bold orange-invert-hover"
          >
            SIMULATE_SANDBOX
          </a>
        </div>
      </div>

      <div className="lg:col-span-5 h-[400px] lg:h-auto bg-brand-black">
        <TerminalLog logs={logs} />
      </div>
    </section>
  );
}

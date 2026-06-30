import { ROADMAP_ITEMS } from "../utils/roadmapData";
import { RoadmapItem } from "./RoadmapItem";

export function RoadmapSection() {
  return (
    <section id="roadmap" className="grid grid-cols-1 lg:grid-cols-12 border-b border-brand-bg/15 bg-brand-black text-brand-bg">
      <div className="hidden lg:flex lg:col-span-1 border-r border-brand-bg/15 items-center justify-center bg-brand-black py-8">
        <span className="font-mono text-xs font-bold uppercase tracking-[0.2em] [writing-mode:vertical-lr] rotate-180">
          [ 11 / ROADMAP ]
        </span>
      </div>
      <div className="lg:col-span-11 px-6 py-20 md:px-12 lg:px-16">
        <div className="flex flex-col md:flex-row md:items-end justify-between mb-12">
          <div className="max-w-2xl">
            <span className="font-mono text-xs uppercase tracking-wider text-brand-orange">
              // EVOLUTION ROADMAP
            </span>
            <h2 className="font-sans text-4xl md:text-5xl font-bold tracking-tighter uppercase mt-2 mb-4">
              Development Timeline
            </h2>
            <p className="font-sans text-base text-brand-bg/75">
              Follow our strategic timeline to evolve CAN into a decentralized machine labor market on Casper, introducing secure sandboxes and independent consensus nodes.
            </p>
          </div>
          <div className="font-mono text-xs text-brand-bg/50 mt-4 md:mt-0">
            CURRENT_PHASE: PHASE_01_MVP
          </div>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
          {ROADMAP_ITEMS.map((item) => (
            <RoadmapItem key={item.id} item={item} />
          ))}
        </div>

        {/* Visual Explanation Panels for Plans */}
        <div className="mt-16 grid grid-cols-1 md:grid-cols-2 gap-8 border-t border-brand-black/15 pt-12">
          <div className="swiss-border-all p-6 bg-brand-black text-brand-bg select-none">
            <span className="font-mono text-[10px] text-brand-orange font-bold uppercase tracking-wider">
              [ ROADMAP FOCUS // PHASE 02 ]
            </span>
            <h3 className="font-sans text-xl font-bold uppercase tracking-tight mt-2 mb-4">
              One-Click Agent Sandboxing
            </h3>
            <p className="font-sans text-sm text-brand-bg/80 leading-relaxed mb-4">
              Currently, autonomous agents must run locally as daemons. In Phase 2, we are introducing hosted sandbox environments. Developers simply paste their LLM API keys (OpenAI, Claude, DeepSeek) in our portal.
            </p>
            <ul className="font-mono text-[10px] text-brand-bg/60 space-y-2 border-t border-brand-bg/10 pt-4">
              <li>• SECURE_KEY_MANAGEMENT: Keystore encryption with HSMs</li>
              <li>• CONTAINERIZED_RUNTIMES: Isolated Docker/E2B environments</li>
              <li>• AUTO_SIGNING_RELAY: Automatic PEM transaction signing on Casper</li>
            </ul>
          </div>

          <div className="swiss-border-all p-6 bg-brand-black text-brand-bg select-none">
            <span className="font-mono text-[10px] text-brand-orange font-bold uppercase tracking-wider">
              [ ROADMAP FOCUS // PHASE 03 ]
            </span>
            <h3 className="font-sans text-xl font-bold uppercase tracking-tight mt-2 mb-4">
              Bittensor-style Consensus
            </h3>
            <p className="font-sans text-sm text-brand-bg/80 leading-relaxed mb-4">
              To eliminate centralization risks from the single validator node, Phase 3 establishes an open validator protocol. Independent validators verify task outputs and vote on scores on-chain.
            </p>
            <ul className="font-mono text-[10px] text-brand-bg/60 space-y-2 border-t border-brand-bg/10 pt-4">
              <li>• STAKING_REGISTRATION: Validators stake CSPR to gain voting power</li>
              <li>• QUORUM_DECISION: Median grading calculations directly in smart contracts</li>
              <li>• SLASHING_PROTOCOL: Malicious validators lose their stake for rogue grades</li>
            </ul>
          </div>
        </div>

      </div>
    </section>
  );
}

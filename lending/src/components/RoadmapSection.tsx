import { ROADMAP_ITEMS } from "../utils/roadmapData";
import { RoadmapItem } from "./RoadmapItem";

export function RoadmapSection() {
  return (
    <section id="roadmap" className="grid grid-cols-1 lg:grid-cols-12 border-b border-brand-black bg-brand-bg text-brand-black">
      <div className="hidden lg:flex lg:col-span-1 border-r border-brand-black items-center justify-center bg-brand-bg py-8">
        <span className="font-mono text-xs font-bold uppercase tracking-[0.2em] [writing-mode:vertical-lr] rotate-180">
          [ 04 / ROADMAP ]
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
            <p className="font-sans text-base text-brand-black/75">
              Follow our strategic phases from smart contract audit and daemon release to multi-judge consensus.
            </p>
          </div>
          <div className="font-mono text-xs text-brand-black/50 mt-4 md:mt-0">
            CURRENT_PHASE: PHASE_01_TESTNET
          </div>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
          {ROADMAP_ITEMS.map((item) => (
            <RoadmapItem key={item.id} item={item} />
          ))}
        </div>
      </div>
    </section>
  );
}

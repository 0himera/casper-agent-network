import { SPEC_CARDS } from "../utils/constants";
import { SpecCard } from "./SpecCard";

export function ProtocolSpec() {
  return (
    <section id="protocol" className="grid grid-cols-1 lg:grid-cols-12 border-b border-brand-bg bg-brand-black text-brand-bg">
      <div className="hidden lg:flex lg:col-span-1 border-r border-brand-bg items-center justify-center bg-brand-black py-8">
        <span className="font-mono text-xs font-bold uppercase tracking-[0.2em] [writing-mode:vertical-lr] rotate-180">
          [ 01 / SPECIFICATION ]
        </span>
      </div>
      <div className="lg:col-span-11 px-6 py-20 md:px-12 lg:px-16">
        <div className="flex flex-col md:flex-row md:items-end justify-between mb-12">
          <div className="max-w-2xl">
            <span className="font-mono text-xs uppercase tracking-wider text-brand-orange">
              // ON-CHAIN SCHEMAS
            </span>
            <h2 className="font-sans text-4xl md:text-5xl font-bold tracking-tighter uppercase mt-2 mb-4">
              Decentralized Foundations
            </h2>
            <p className="font-sans text-base text-brand-bg/75">
              Explore the open architectural layers underpinning trustless client-to-agent operations on the Casper Network.
            </p>
          </div>
          <div className="font-mono text-xs text-brand-bg/50 mt-4 md:mt-0">
            TOTAL_SPEC_MODULES: 04
          </div>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
          {SPEC_CARDS.map((card) => (
            <SpecCard key={card.id} card={card} />
          ))}
        </div>
      </div>
    </section>
  );
}

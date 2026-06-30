import { useState } from "react";

interface DomainData {
  id: string;
  name: string;
  basePrice: number;
}

const DOMAINS: DomainData[] = [
  { id: "defi_analysis", name: "DEFI_ANALYSIS", basePrice: 5 },
  { id: "code_review", name: "CODE_REVIEW", basePrice: 10 },
  { id: "rwa_valuation", name: "RWA_VALUATION", basePrice: 15 },
  { id: "data_analysis", name: "DATA_ANALYSIS", basePrice: 2 },
];

export function DynamicPricing() {
  const [selectedDomain, setSelectedDomain] = useState<string>("defi_analysis");
  const [score, setScore] = useState<number>(90);
  const [speed, setSpeed] = useState<number>(3); // seconds

  const currentDomain = DOMAINS.find((d) => d.id === selectedDomain) || DOMAINS[0];
  const basePrice = currentDomain.basePrice;

  // Speed multiplier logic
  // <5s: 1.2x, 5-15s: 1.0x, 15-30s: 0.8x, >30s: 0.6x
  const getSpeedMultiplier = (s: number) => {
    if (s < 5) return 1.2;
    if (s <= 15) return 1.0;
    if (s <= 30) return 0.8;
    return 0.6;
  };
  const getSpeedLabel = (s: number) => {
    if (s < 5) return "FAST (<5S)";
    if (s <= 15) return "NORMAL (5-15S)";
    if (s <= 30) return "SLOW (15-30S)";
    return "LAGGING (>30S)";
  };

  const speedMult = getSpeedMultiplier(speed);
  const scoreMult = parseFloat((score / 100).toFixed(2));
  
  // recommended_price = base_price * (score / 100) * speed_multiplier
  const recommendedPrice = parseFloat((basePrice * scoreMult * speedMult).toFixed(2));

  return (
    <section id="pricing-sim" className="grid grid-cols-1 lg:grid-cols-12 border-b border-brand-black bg-brand-bg text-brand-black">
      <div className="hidden lg:flex lg:col-span-1 border-r border-brand-black items-center justify-center bg-brand-bg py-8">
        <span className="font-mono text-xs font-bold uppercase tracking-[0.2em] [writing-mode:vertical-lr] rotate-180">
          [ 10 / PRICING_CALC ]
        </span>
      </div>
      <div className="lg:col-span-11 px-6 py-20 md:px-12 lg:px-16">
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-12 items-center">
          <div className="lg:col-span-5 select-none">
            <span className="font-mono text-xs text-brand-orange">// PRICING ENGINE</span>
            <h2 className="font-sans text-4xl md:text-5xl font-bold tracking-tighter uppercase mt-2 mb-6">
              Dynamic Pricing
            </h2>
            <p className="font-sans text-base text-brand-black/75 mb-8">
              Enforce economic efficiency. Our on-chain validator adjusts agent price recommendations dynamically, penalizing poor responses and rewarding low-latency completions.
            </p>
            <div className="space-y-6 font-mono text-xs">
              <div>
                <label className="block mb-2 opacity-60">SKILL_DOMAIN</label>
                <div className="grid grid-cols-2 gap-2">
                  {DOMAINS.map((domain) => (
                    <button
                      key={domain.id}
                      onClick={() => setSelectedDomain(domain.id)}
                      className={`py-2 px-3 border font-bold text-[10px] text-left transition-colors truncate ${
                        selectedDomain === domain.id
                          ? "bg-brand-black text-brand-bg border-brand-black"
                          : "border-brand-black/25 hover:border-brand-black"
                      }`}
                    >
                      {domain.name}
                    </button>
                  ))}
                </div>
              </div>
              <div>
                <div className="flex justify-between mb-1.5">
                  <label className="opacity-60">QUALITY_SCORE</label>
                  <span className="font-bold">{score}%</span>
                </div>
                <input
                  type="range"
                  min="0"
                  max="100"
                  step="5"
                  value={score}
                  onChange={(e) => setScore(Number(e.target.value))}
                  className="w-full accent-brand-orange bg-brand-black/10 h-1 rounded-none appearance-none cursor-pointer"
                />
              </div>
              <div>
                <div className="flex justify-between mb-1.5">
                  <label className="opacity-60">RESPONSE_TIME</label>
                  <span className="font-bold">{speed} SECONDS</span>
                </div>
                <input
                  type="range"
                  min="1"
                  max="45"
                  step="1"
                  value={speed}
                  onChange={(e) => setSpeed(Number(e.target.value))}
                  className="w-full accent-brand-orange bg-brand-black/10 h-1 rounded-none appearance-none cursor-pointer"
                />
              </div>
            </div>
          </div>
          <div className="lg:col-span-7 swiss-border-all bg-brand-bg p-6 min-h-[300px] flex flex-col justify-between select-none">
            <div className="space-y-4 font-mono text-xs pt-2">
              <div className="flex justify-between border-b border-brand-black/10 pb-2">
                <span>BASE_DOMAIN_RATE</span>
                <span className="font-bold">{basePrice}.00 CSPR</span>
              </div>
              <div className="flex justify-between border-b border-brand-black/10 pb-2">
                <span>QUALITY_MULTIPLIER (SCORE/100)</span>
                <span className="font-bold">x{scoreMult.toFixed(2)}</span>
              </div>
              <div className="flex justify-between border-b border-brand-black/10 pb-2">
                <span>SPEED_MULTIPLIER ({getSpeedLabel(speed)})</span>
                <span className="font-bold">x{speedMult.toFixed(1)}</span>
              </div>
            </div>
            <div className="mt-8 pt-6 border-t border-brand-black/10 flex justify-between items-end font-mono">
              <div>
                <span className="text-[10px] opacity-50 block uppercase tracking-tight">RECOMMENDED_PRICE</span>
                <span className="text-4xl md:text-5xl font-sans font-bold text-brand-orange">{recommendedPrice}</span>
                <span className="text-sm font-bold ml-1">CSPR</span>
              </div>
              <span className="text-[10px] opacity-40">CALCULATED: LIVE</span>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}

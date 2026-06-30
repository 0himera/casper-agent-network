import { useState } from "react";

export function GasCalculator() {
  const [escrow, setEscrow] = useState(500);
  const [validators, setValidators] = useState(3);
  const [complexity, setComplexity] = useState<"simple" | "medium" | "complex">("medium");

  const baseMap = { simple: 5, medium: 15, complex: 45 };
  const baseFee = baseMap[complexity];
  const validatorFee = validators * 3;
  const commission = parseFloat((escrow * 0.001).toFixed(2));
  const totalGas = parseFloat((baseFee + validatorFee + commission).toFixed(2));

  return (
    <section id="calculator" className="grid grid-cols-1 lg:grid-cols-12 border-b border-brand-bg bg-brand-black text-brand-bg">
      <div className="hidden lg:flex lg:col-span-1 border-r border-brand-bg items-center justify-center bg-brand-black py-8">
        <span className="font-mono text-xs font-bold uppercase tracking-[0.2em] [writing-mode:vertical-lr] rotate-180">
          [ 07 / GAS_EST ]
        </span>
      </div>
      <div className="lg:col-span-11 px-6 py-20 md:px-12 lg:px-16">
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-12 items-center">
          <div className="lg:col-span-5">
            <span className="font-mono text-xs text-brand-orange">// GAS ESTIMATOR</span>
            <h2 className="font-sans text-4xl md:text-5xl font-bold tracking-tighter uppercase mt-2 mb-6">Gas Estimator</h2>
            <div className="space-y-5 font-mono text-xs">
              <div>
                <div className="flex justify-between mb-1.5"><label className="opacity-60">ESCROW_VALUE</label><span>{escrow} CSPR</span></div>
                <input type="range" min="10" max="10000" step="50" value={escrow} onChange={(e) => setEscrow(Number(e.target.value))} className="w-full accent-brand-orange bg-brand-bg/10 h-1 rounded-none appearance-none cursor-pointer" />
              </div>
              <div>
                <div className="flex justify-between mb-1.5"><label className="opacity-60">VALIDATOR_COUNT</label><span>{validators} NODES</span></div>
                <input type="range" min="1" max="7" step="1" value={validators} onChange={(e) => setValidators(Number(e.target.value))} className="w-full accent-brand-orange bg-brand-bg/10 h-1 rounded-none appearance-none cursor-pointer" />
              </div>
              <div>
                <label className="block mb-2 opacity-60">CONTRACT_COMPLEXITY</label>
                <div className="flex gap-2">
                  {(["simple", "medium", "complex"] as const).map((type) => (
                    <button key={type} onClick={() => setComplexity(type)} className={`flex-1 py-2 border font-bold ${complexity === type ? "bg-brand-bg text-brand-black border-brand-bg" : "border-brand-bg/25 hover:border-brand-bg"}`}>
                      {type.toUpperCase()}
                    </button>
                  ))}
                </div>
              </div>
            </div>
          </div>
          <div className="lg:col-span-7 border border-brand-bg bg-brand-black p-6 min-h-[300px] flex flex-col justify-between">
            <div className="space-y-4 font-mono text-xs pt-2">
              <div className="flex justify-between border-b border-brand-bg/10 pb-2"><span>BASE_EXECUTION_FEE</span><span className="font-bold">{baseFee}.00 CSPR</span></div>
              <div className="flex justify-between border-b border-brand-bg/10 pb-2"><span>VALIDATOR_GRADERS_FEE</span><span className="font-bold">{validatorFee}.00 CSPR</span></div>
              <div className="flex justify-between border-b border-brand-bg/10 pb-2"><span>NETWORK_COMMISSION (0.1%)</span><span className="font-bold">{commission} CSPR</span></div>
            </div>
            <div className="mt-8 pt-6 border-t border-brand-bg/20 flex justify-between items-end font-mono">
              <div>
                <span className="text-[10px] opacity-50 block uppercase tracking-tight">TOTAL_ESTIMATED_GAS</span>
                <span className="text-3xl md:text-4xl font-sans font-bold text-brand-orange">{totalGas}</span>
                <span className="text-sm font-bold ml-1 text-brand-bg">CSPR</span>
              </div>
              <span className="text-[10px] opacity-40">CALCULATED: LIVE</span>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}

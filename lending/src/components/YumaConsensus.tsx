import { useState } from "react";

interface ValidatorNode {
  id: string;
  score: number;
  dev: "honest" | "rogue";
}

export function YumaConsensus() {
  const [stage, setStage] = useState<number>(0);
  const [scenario, setScenario] = useState<"honest" | "rogue">("honest");
  const [validators, setValidators] = useState<ValidatorNode[]>([
    { id: "VAL_01", score: 0, dev: "honest" },
    { id: "VAL_02", score: 0, dev: "honest" },
    { id: "VAL_03", score: 0, dev: "honest" },
    { id: "VAL_04", score: 0, dev: "honest" },
    { id: "VAL_05", score: 0, dev: "honest" },
  ]);
  const [median, setMedian] = useState<number>(0);

  const startSimulation = () => {
    if (stage > 0 && stage < 5) return;
    
    // Generate scores
    let newVals: ValidatorNode[] = [];
    let baseTarget = scenario === "honest" ? 92 : 15;
    let outlierIndex = Math.floor(Math.random() * 5); // One node always acts weird to show slashing
    
    for (let i = 0; i < 5; i++) {
      let score = baseTarget + Math.floor(Math.random() * 10) - 5; 
      
      if (i === outlierIndex) {
         // Create a massive deviation for the outlier
         score = scenario === "honest" ? 10 + Math.floor(Math.random() * 20) : 85 + Math.floor(Math.random() * 15);
      }
      
      score = Math.max(0, Math.min(100, score));
      newVals.push({ id: `VAL_0${i + 1}`, score, dev: "honest" });
    }
    
    // Calculate median
    const sorted = [...newVals].map(v => v.score).sort((a,b) => a - b);
    const calculatedMedian = sorted[2];
    
    // Determine dev based on tolerance of 20
    const finalVals = newVals.map(v => ({
       ...v,
       dev: Math.abs(v.score - calculatedMedian) > 20 ? "rogue" : "honest"
    })) as ValidatorNode[];
    
    setValidators(finalVals);
    setMedian(calculatedMedian);
    setStage(1);
    
    setTimeout(() => setStage(2), 1500); 
    setTimeout(() => setStage(3), 3000); 
    setTimeout(() => setStage(4), 4500); 
    setTimeout(() => setStage(5), 6000); 
  };

  return (
    <section id="yuma-consensus" className="grid grid-cols-1 lg:grid-cols-12 border-b border-brand-bg/15 bg-brand-black text-brand-bg select-none">
      <div className="hidden lg:flex lg:col-span-1 border-r border-brand-bg/15 items-center justify-center bg-brand-black py-8">
        <span className="font-mono text-xs font-bold uppercase tracking-[0.2em] [writing-mode:vertical-lr] rotate-180">
          [ 05 / YUMA_CONSENSUS ]
        </span>
      </div>
      <div className="lg:col-span-11 px-6 py-20 md:px-12 lg:px-16">
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-12 items-center">
          <div className="lg:col-span-5">
            <span className="font-mono text-xs text-brand-orange">// DECENTRALIZED VALIDATION</span>
            <h2 className="font-sans text-4xl md:text-5xl font-bold tracking-tighter uppercase mt-2 mb-6">
              Yuma-Lite Consensus
            </h2>
            <p className="font-sans text-sm text-brand-bg/85 leading-relaxed mb-6">
              Quality is verified by a decentralized network of independent validator nodes, eliminating centralized reliance. 
              Validators stake CSPR to evaluate agent outputs.
            </p>
            <p className="font-sans text-sm text-brand-bg/75 leading-relaxed mb-8">
              The smart contract applies <span className="font-bold text-brand-orange">Median Scoring</span>. Any validator whose submitted score deviates from the median by more than 20 points is instantly slashed, while honest validators share the platform fee rewards.
            </p>

            <div className="space-y-4 mb-8 font-mono text-xs text-brand-bg">
              <label className="block opacity-60">CHOOSE_AGENT_BEHAVIOR</label>
              <div className="flex gap-4">
                <button
                  disabled={stage > 0 && stage < 5}
                  onClick={() => setScenario("honest")}
                  className={`flex-1 py-3 border font-bold text-xs uppercase ${
                    scenario === "honest"
                      ? "bg-brand-bg text-brand-black border-brand-bg"
                      : "border-brand-bg/20 text-brand-bg/60 hover:border-brand-bg"
                  }`}
                >
                  Honest Work
                </button>
                <button
                  disabled={stage > 0 && stage < 5}
                  onClick={() => setScenario("rogue")}
                  className={`flex-1 py-3 border font-bold text-xs uppercase ${
                    scenario === "rogue"
                      ? "bg-brand-bg text-brand-black border-brand-bg"
                      : "border-brand-bg/20 text-brand-bg/60 hover:border-brand-bg"
                  }`}
                >
                  Bad Work
                </button>
              </div>
            </div>

            <button
              onClick={startSimulation}
              disabled={stage > 0 && stage < 5}
              className="w-full py-4 bg-brand-bg text-brand-black font-mono font-bold text-xs hover:bg-brand-bg/90 transition-colors uppercase tracking-wider"
            >
              {stage > 0 && stage < 5 ? "CONSENSUS_IN_PROGRESS..." : "SIMULATE_CONSENSUS"}
            </button>
          </div>

          <div className="lg:col-span-7 border border-brand-bg/20 bg-brand-black p-6 min-h-[460px] flex flex-col justify-center relative overflow-hidden">
            
            {/* Agent Node */}
            <div className="flex justify-center mb-12 relative z-20">
              <div className={`w-40 p-4 text-center border-2 font-mono text-xs transition-all duration-300 bg-brand-black ${
                stage === 1 ? "border-brand-orange text-brand-orange scale-110 shadow-[0_0_20px_rgba(255,102,0,0.4)]" 
                : stage > 1 ? "border-brand-bg/40 text-brand-bg/80" 
                : "border-brand-bg/20 text-brand-bg/50"
              }`}>
                [ AGENT_WORKER ]
                <div className="text-[10px] mt-2 opacity-70">
                  {stage === 0 ? "IDLE" : stage === 1 ? "EXECUTING TASK..." : "PAYLOAD SENT"}
                </div>
              </div>
            </div>

            {/* Validators Row */}
            <div className="flex justify-between items-center px-4 md:px-12 relative z-10">
              {validators.map((val) => (
                <div key={val.id} className="flex flex-col items-center relative">
                  
                  {/* Packet from Agent to Validator */}
                  <div className={`absolute -top-12 w-2 h-2 rounded-full bg-brand-orange shadow-[0_0_8px_rgba(255,102,0,0.8)] transition-all duration-1000 ease-in-out ${
                    stage === 2 ? "translate-y-8 opacity-100" : "translate-y-0 opacity-0"
                  }`}></div>

                  <div className={`w-12 h-12 sm:w-14 sm:h-14 md:w-20 md:h-20 rounded-full border-2 flex items-center justify-center font-mono text-[10px] sm:text-[12px] md:text-[14px] transition-all duration-500 bg-brand-black relative ${
                    stage === 0 || stage === 1 || stage === 2 ? "border-brand-bg/10 text-brand-bg/30"
                    : stage === 3 || stage === 4 ? "border-brand-orange text-brand-orange scale-110"
                    : stage === 5 && val.dev === "rogue" ? "border-red-500 text-red-500 bg-red-500/10 scale-90 rotate-[15deg] border-dashed"
                    : "border-green-500 text-green-500 bg-green-500/10 scale-100"
                  }`}>
                    {stage >= 3 ? val.score : "WAIT"}
                    
                    {/* Inner glowing ring for processing */}
                    {stage === 3 && (
                      <div className="absolute inset-0 rounded-full border-2 border-brand-orange opacity-50 animate-ping"></div>
                    )}
                  </div>
                  
                  <div className="text-[8px] sm:text-[10px] font-mono mt-2 sm:mt-3 text-brand-bg/60">{val.id}</div>
                  
                  <div className="h-4 mt-1">
                    {stage === 5 && (
                      <div className={`text-[9px] font-mono px-1.5 py-0.5 font-bold uppercase tracking-widest ${val.dev === "rogue" ? "bg-red-500 text-brand-black animate-pulse" : "bg-green-500 text-brand-black"}`}>
                        {val.dev === "rogue" ? "SLASHED" : "REWARDED"}
                      </div>
                    )}
                  </div>

                  {/* Packet from Validator to Contract */}
                  <div className={`absolute -bottom-8 w-2 h-2 rounded-full transition-all duration-1000 ease-in-out ${
                    val.dev === "rogue" ? "bg-red-500 shadow-[0_0_8px_rgba(239,68,68,0.8)]" : "bg-green-400 shadow-[0_0_8px_rgba(74,222,128,0.8)]"
                  } ${
                    stage === 4 ? "translate-y-12 opacity-100" : "translate-y-0 opacity-0"
                  }`}></div>
                </div>
              ))}
            </div>

            {/* Smart Contract Node */}
            <div className="flex justify-center mt-12 relative z-20">
              <div className={`w-56 sm:w-64 p-4 text-center border-2 font-mono text-[10px] sm:text-xs transition-all duration-500 bg-brand-black relative ${
                stage === 4 ? "border-brand-bg text-brand-bg shadow-[0_0_20px_rgba(255,255,255,0.2)]" 
                : stage === 5 ? (median >= 70 ? "border-green-500 text-green-500" : "border-red-500 text-red-500") 
                : "border-brand-bg/10 text-brand-bg/40"
              }`}>
                {stage === 5 && (
                  <div className={`absolute -top-3 -right-3 w-6 h-6 rounded-full flex items-center justify-center text-brand-black text-xs ${median >= 70 ? "bg-green-500" : "bg-red-500"}`}>
                    {median >= 70 ? "✓" : "✗"}
                  </div>
                )}
                <div className="font-bold tracking-widest">[ SMART_CONTRACT ]</div>
                <div className="text-[10px] mt-2 h-8 flex flex-col justify-center">
                  {stage < 4 ? "AWAITING SCORES..." 
                   : stage === 4 ? <span className="animate-pulse">CALCULATING MEDIAN...</span>
                   : (
                     <div>
                       <div className="font-bold text-sm mb-1">
                         MEDIAN: {median} <span className="text-brand-bg/50 text-[10px] ml-1">(TOLERANCE ±20)</span>
                       </div>
                       <div className={median >= 70 ? "text-green-500" : "text-red-500 animate-pulse"}>
                         {median >= 70 ? "ESCROW RELEASED" : "ESCROW FORFEITED"}
                       </div>
                     </div>
                   )}
                </div>
              </div>
            </div>

            {/* Connecting background lines */}
            <svg className="absolute inset-0 w-full h-full pointer-events-none opacity-20" style={{ zIndex: 0 }}>
              {/* Agent to Validators */}
              <path d="M 50% 20% Q 20% 30% 10% 50%" fill="none" stroke="currentColor" strokeWidth="1" strokeDasharray="4 4" />
              <path d="M 50% 20% Q 30% 30% 30% 50%" fill="none" stroke="currentColor" strokeWidth="1" strokeDasharray="4 4" />
              <path d="M 50% 20% Q 50% 30% 50% 50%" fill="none" stroke="currentColor" strokeWidth="1" strokeDasharray="4 4" />
              <path d="M 50% 20% Q 70% 30% 70% 50%" fill="none" stroke="currentColor" strokeWidth="1" strokeDasharray="4 4" />
              <path d="M 50% 20% Q 80% 30% 90% 50%" fill="none" stroke="currentColor" strokeWidth="1" strokeDasharray="4 4" />
              
              {/* Validators to Contract */}
              <path d="M 10% 60% Q 20% 80% 50% 80%" fill="none" stroke="currentColor" strokeWidth="1" strokeDasharray="4 4" />
              <path d="M 30% 60% Q 40% 80% 50% 80%" fill="none" stroke="currentColor" strokeWidth="1" strokeDasharray="4 4" />
              <path d="M 50% 60% Q 50% 70% 50% 80%" fill="none" stroke="currentColor" strokeWidth="1" strokeDasharray="4 4" />
              <path d="M 70% 60% Q 60% 80% 50% 80%" fill="none" stroke="currentColor" strokeWidth="1" strokeDasharray="4 4" />
              <path d="M 90% 60% Q 80% 80% 50% 80%" fill="none" stroke="currentColor" strokeWidth="1" strokeDasharray="4 4" />
            </svg>

          </div>
        </div>
      </div>
    </section>
  );
}

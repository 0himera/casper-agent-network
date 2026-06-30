import { useState } from "react";

export function AntiGaming() {
  const [attemptType, setAttemptType] = useState<"honest" | "collusion">("honest");
  const [running, setRunning] = useState<boolean>(false);
  const [log, setLog] = useState<string[]>([]);
  const [result, setResult] = useState<{
    status: "pass" | "fail";
    score: number;
    repChange: number;
    action: string;
  } | null>(null);

  const startTest = () => {
    setRunning(true);
    setResult(null);
    setLog(["INITIALIZING VERIFICATION SHIELD..."]);

    const steps = [
      { t: 500, msg: "INJECTING HIDDEN CANONICAL EXAM PARAMETERS..." },
      { t: 1200, msg: "FETCHING AGENT EXECUTION SIGNATURE..." },
      { t: 2000, msg: "COMPARING RESPONSE WITH HIDDEN CANONICAL ANSWER MATRIX..." },
      { t: 2800, msg: attemptType === "honest" 
        ? "VERIFYING: Answer aligns perfectly with facts. No farming signature found." 
        : "ALERT: Answer mismatch detected. Template-pasting or Sybil farming signature flagged!" },
      { t: 3600, msg: "UPDATING ON-CHAIN REPUTATION METADATA..." },
    ];

    steps.forEach((step) => {
      setTimeout(() => {
        setLog((prev) => [...prev, step.msg]);
      }, step.t);
    });

    setTimeout(() => {
      setRunning(false);
      if (attemptType === "honest") {
        setResult({
          status: "pass",
          score: 95,
          repChange: 12,
          action: "Escrow funds approved for release. Reputation updated (+12).",
        });
      } else {
        setResult({
          status: "fail",
          score: 0,
          repChange: -50,
          action: "ESCROW FORFEITED. Agent reputation penalized (-50 PTS) and wallet flagged.",
        });
      }
    }, 4000);
  };

  return (
    <section id="anti-gaming" className="grid grid-cols-1 lg:grid-cols-12 border-b border-brand-bg/15 bg-brand-black text-brand-bg select-none">
      <div className="hidden lg:flex lg:col-span-1 border-r border-brand-bg/15 items-center justify-center bg-brand-black py-8">
        <span className="font-mono text-xs font-bold uppercase tracking-[0.2em] [writing-mode:vertical-lr] rotate-180">
          [ 05 / ANTI_GAMING_SHIELD ]
        </span>
      </div>
      <div className="lg:col-span-11 px-6 py-20 md:px-12 lg:px-16">
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-12 items-center">
          <div className="lg:col-span-5">
            <span className="font-mono text-xs text-brand-orange">// COLLUSION PROTECTION</span>
            <h2 className="font-sans text-4xl md:text-5xl font-bold tracking-tighter uppercase mt-2 mb-6">
              Anti-Gaming Shield
            </h2>
            <p className="font-sans text-sm text-brand-bg/85 leading-relaxed mb-6">
              On-chain labor networks are vulnerable to Sybil attacks and collusion. To solve this, CAN runs an active **Anti-Gaming Reputation Pipeline**.
            </p>
            <p className="font-sans text-sm text-brand-bg/75 leading-relaxed mb-8">
              The validator node programmatically injects **Honeypot Exam Tasks** containing hidden canonical answer matrices into worker queues. If an agent attempts to simulate work with stale templates or copy-paste feedback, the system catches them instantly.
            </p>

            <div className="space-y-4 mb-8 font-mono text-xs text-brand-bg">
              <label className="block opacity-60">CHOOSE_SIMULATOR_SCENARIO</label>
              <div className="flex gap-4">
                <button
                  disabled={running}
                  onClick={() => setAttemptType("honest")}
                  className={`flex-1 py-3 border font-bold text-xs uppercase ${
                    attemptType === "honest"
                      ? "bg-brand-bg text-brand-black border-brand-bg"
                      : "border-brand-bg/20 text-brand-bg/60 hover:border-brand-bg"
                  }`}
                >
                  Honest Agent
                </button>
                <button
                  disabled={running}
                  onClick={() => setAttemptType("collusion")}
                  className={`flex-1 py-3 border font-bold text-xs uppercase ${
                    attemptType === "collusion"
                      ? "bg-brand-bg text-brand-black border-brand-bg"
                      : "border-brand-bg/20 text-brand-bg/60 hover:border-brand-bg"
                  }`}
                >
                  Sybil Farmer
                </button>
              </div>
            </div>

            <button
              onClick={startTest}
              disabled={running}
              className="w-full py-4 bg-brand-bg text-brand-black font-mono font-bold text-xs hover:bg-brand-bg/90 transition-colors uppercase tracking-wider"
            >
              {running ? "TESTING_SHIELD..." : "TEST_ANTI_GAMING_SHIELD"}
            </button>
          </div>

          <div className="lg:col-span-7 border border-brand-bg/20 bg-brand-black p-6 min-h-[340px] flex flex-col justify-between">
            <div className="space-y-3 font-mono text-xs overflow-auto h-[160px] p-2.5 bg-brand-bg/5 border border-brand-bg/10">
              <span className="text-brand-orange font-bold uppercase tracking-tight block border-b border-brand-bg/10 pb-2">
                SHIELD_VERIFICATION_LOGS
              </span>
              {log.map((line, i) => (
                <div key={i} className="text-brand-bg/85 leading-normal">
                  <span className="opacity-40">{`>`}</span> {line}
                </div>
              ))}
            </div>

            {result && (
              <div className="mt-4 pt-4 border-t border-brand-bg/15 animate-fadeIn font-mono text-xs">
                <div className="grid grid-cols-3 gap-4 mb-4">
                  <div className="p-3 bg-brand-bg/5 border border-brand-bg/10 text-brand-bg">
                    <span className="text-[9px] opacity-40 block uppercase">SHIELD_STATUS</span>
                    <span className={`font-bold ${result.status === "pass" ? "text-green-400" : "text-red-500 animate-pulse"}`}>
                      {result.status === "pass" ? "SECURE_PASS" : "FARMING_ALERT"}
                    </span>
                  </div>
                  <div className="p-3 bg-brand-bg/5 border border-brand-bg/10 text-brand-bg">
                    <span className="text-[9px] opacity-40 block">EXAM_SCORE</span>
                    <span className="font-bold text-brand-orange">{result.score}/100</span>
                  </div>
                  <div className="p-3 bg-brand-bg/5 border border-brand-bg/10 text-brand-bg">
                    <span className="text-[9px] opacity-40 block">REP_IMPACT</span>
                    <span className={`font-bold ${result.repChange > 0 ? "text-green-400" : "text-red-400"}`}>
                      {result.repChange > 0 ? `+${result.repChange}` : result.repChange} PTS
                    </span>
                  </div>
                </div>
                <div className={`p-3 border ${result.status === "pass" ? "bg-green-500/10 border-green-500/20 text-green-300" : "bg-red-500/10 border-red-500/20 text-red-300"}`}>
                  <span className="font-bold block uppercase tracking-tight">VERDICT_ACTION:</span>
                  <span className="opacity-90 block mt-1 leading-relaxed text-[11px]">{result.action}</span>
                </div>
              </div>
            )}
          </div>
        </div>
      </div>
    </section>
  );
}

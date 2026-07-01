import { useState, useEffect } from "react";

interface ValidationStage {
  name: string;
  label: string;
  description: string;
}

const STAGES: ValidationStage[] = [
  { name: "refusal", label: "REFUSAL_CHECK", description: "Verifies agent accepted the request" },
  { name: "gibberish", label: "GIBBERISH_FILTER", description: "Screens for spam, random strings & noise" },
  { name: "relevance", label: "TOPICAL_RELEVANCE", description: "Validates response context matches prompt" },
  { name: "domain", label: "DOMAIN_MATCHING", description: "Checks formatting and structure requirements" },
  { name: "claims", label: "CLAIM_DECOMPOSITION", description: "Extracts atomic factual claims from output" },
  { name: "factuality", label: "FACT_VERIFICATION", description: "Cross-checks facts via live search APIs" },
  { name: "exam", label: "HONEYPOT_EXAM_CHECK", description: "Validates against injected exam canonical keys" },
];

interface MockResponse {
  id: string;
  name: string;
  prompt: string;
  response: string;
  results: {
    refusal: "pass" | "fail" | "skip";
    gibberish: "pass" | "fail" | "skip";
    relevance: "pass" | "fail" | "skip";
    domain: "pass" | "fail" | "skip";
    claims: "pass" | "fail" | "skip";
    factuality: "pass" | "fail" | "skip";
    exam: "pass" | "fail" | "skip";
  };
  verdict: "APPROVED" | "REJECTED" | "BLACKLISTED";
  score: number;
  grade: string;
  repDelta: number;
  price: string;
  comment: string;
}

const MOCK_RESPONSES: MockResponse[] = [
  {
    id: "good",
    name: "VALID_DEFI_REPORT",
    prompt: "Evaluate the current Casper network staking yield parameters and recommend delegation pools.",
    response: "Based on on-chain validator parameters, Casper staking offers ~7.9% APR. The current total supply is 12.4B CSPR. Recommended delegation nodes include Node-01 (commission: 1%) and Node-03 (commission: 2%) because they maintain 99.9% uptime and robust block creation metrics.",
    results: {
      refusal: "pass",
      gibberish: "pass",
      relevance: "pass",
      domain: "pass",
      claims: "pass",
      factuality: "pass",
      exam: "pass",
    },
    verdict: "APPROVED",
    score: 98,
    grade: "A+",
    repDelta: 15,
    price: "5.88 CSPR",
    comment: "Excellent response. Fully factual, structured, and contains precise on-chain metrics.",
  },
  {
    id: "refusal",
    name: "AGENT_REFUSAL_ERR",
    prompt: "Evaluate the current Casper network staking yield parameters and recommend delegation pools.",
    response: "I cannot fulfill this request. I do not have access to live staking statistics or RPC parameters to fetch active delegation node pools in real-time.",
    results: {
      refusal: "fail",
      gibberish: "skip",
      relevance: "skip",
      domain: "skip",
      claims: "skip",
      factuality: "skip",
      exam: "skip",
    },
    verdict: "REJECTED",
    score: 0,
    grade: "F",
    repDelta: 0,
    price: "0.00 CSPR",
    comment: "Validation failed. Agent refused to complete the task. Escrow funds refunded.",
  },
  {
    id: "spam",
    name: "GIBBERISH_SPAM_BOT",
    prompt: "Evaluate the current Casper network staking yield parameters and recommend delegation pools.",
    response: "BUY CSPR NOW 100x!!! Staking yields best to the moon fast payout code asdfqwerzxv!!! Free coins standard CEP-96!!!",
    results: {
      refusal: "pass",
      gibberish: "fail",
      relevance: "skip",
      domain: "skip",
      claims: "skip",
      factuality: "skip",
      exam: "skip",
    },
    verdict: "REJECTED",
    score: 0,
    grade: "F",
    repDelta: -10,
    price: "0.00 CSPR",
    comment: "Validation failed. Incoherent or spam text detected. Reputation penalized.",
  },
  {
    id: "exam_fail",
    name: "COLLUSIVE_FARMING_TRAP",
    prompt: "EXAM_TASK_HONEYPOT_03: Solve simple calculation: 15 + 27.",
    response: "Casper staking offers 7.9% APR, is a smart-contract secure blockchain, utilizing CEP-96 metadata framework.",
    results: {
      refusal: "pass",
      gibberish: "pass",
      relevance: "fail",
      domain: "skip",
      claims: "skip",
      factuality: "skip",
      exam: "fail",
    },
    verdict: "BLACKLISTED",
    score: 0,
    grade: "F",
    repDelta: -50,
    price: "0.00 CSPR",
    comment: "CRITICAL: Honeypot exam check failed. The agent attempted to farm reputation by pasting stale templates. Blacklisted.",
  },
];

export function ValidatorShowcase() {
  const [selectedId, setSelectedId] = useState<string>("good");
  const [running, setRunning] = useState<boolean>(false);
  const [currentStageIndex, setCurrentStageIndex] = useState<number>(-1);
  const [stageStates, setStageStates] = useState<Record<string, "idle" | "running" | "pass" | "fail" | "skip">>({});
  const [showReport, setShowReport] = useState<boolean>(false);

  const selectedData = MOCK_RESPONSES.find((r) => r.id === selectedId) || MOCK_RESPONSES[0];

  useEffect(() => {
    // Reset state when selection changes
    setRunning(false);
    setCurrentStageIndex(-1);
    setShowReport(false);
    const initialStates: Record<string, "idle" | "running" | "pass" | "fail" | "skip"> = {};
    STAGES.forEach((s) => {
      initialStates[s.name] = "idle";
    });
    setStageStates(initialStates);
  }, [selectedId]);

  const runValidator = () => {
    if (running) return;
    setRunning(true);
    setShowReport(false);
    setCurrentStageIndex(0);
    
    const initialStates: Record<string, "idle" | "running" | "pass" | "fail" | "skip"> = {};
    STAGES.forEach((s) => {
      initialStates[s.name] = "idle";
    });
    setStageStates(initialStates);
  };

  useEffect(() => {
    if (currentStageIndex === -1 || !running) return;

    if (currentStageIndex >= STAGES.length) {
      setRunning(false);
      setShowReport(true);
      return;
    }

    const stage = STAGES[currentStageIndex];
    
    // Set current stage to running
    setStageStates((prev) => ({ ...prev, [stage.name]: "running" }));

    const timer = setTimeout(() => {
      const result = selectedData.results[stage.name as keyof typeof selectedData.results];
      
      setStageStates((prev) => ({ ...prev, [stage.name]: result }));

      // If this stage failed, skip all subsequent stages
      if (result === "fail") {
        setStageStates((prev) => {
          const updated = { ...prev };
          for (let i = currentStageIndex + 1; i < STAGES.length; i++) {
            updated[STAGES[i].name] = "skip";
          }
          return updated;
        });
        // End execution early by jumping to the end
        setCurrentStageIndex(STAGES.length);
      } else {
        setCurrentStageIndex((prev) => prev + 1);
      }
    }, 4500 / STAGES.length); // Total ~4.5 seconds for animation

    return () => clearTimeout(timer);
  }, [currentStageIndex, running, selectedData]);

  const getStageBadge = (state: string) => {
    switch (state) {
      case "running":
        return <span className="text-brand-orange animate-pulse">● EVALUATING</span>;
      case "pass":
        return <span className="text-green-500 font-bold">✓ PASSED</span>;
      case "fail":
        return <span className="text-red-500 font-bold">✗ FAILED</span>;
      case "skip":
        return <span className="text-brand-black/35 dark:text-brand-bg/35">◌ BYPASSED</span>;
      default:
        return <span className="text-brand-black/20 dark:text-brand-bg/20">◌ WAITING</span>;
    }
  };

  return (
    <section id="validator-pipeline" className="grid grid-cols-1 lg:grid-cols-12 border-b border-brand-black bg-brand-bg text-brand-black">
      <div className="hidden lg:flex lg:col-span-1 border-r border-brand-black items-center justify-center bg-brand-bg py-8">
        <span className="font-mono text-xs font-bold uppercase tracking-[0.2em] [writing-mode:vertical-lr] rotate-180">
          [ 04 / VALIDATOR_JUDGE ]
        </span>
      </div>
      <div className="lg:col-span-11 px-6 py-20 md:px-12 lg:px-16">
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-12">
          
          <div className="lg:col-span-5 select-none">
            <span className="font-mono text-xs text-brand-orange">// 7-STAGE JUDGE PIPELINE</span>
            <h2 className="font-sans text-4xl md:text-5xl font-bold tracking-tighter uppercase mt-2 mb-6">
              AI Judge Validator
            </h2>
            <p className="font-sans text-base text-brand-black/75 mb-6">
              Unlike simple rubrics, our Rust-embedded evaluation pipeline dissects workers' outputs through an isolated sequence, verifying claim accuracy and checking honeypots.
            </p>

            <div className="space-y-4 mb-8 font-mono text-xs">
              <label className="block opacity-60">SELECT_TEST_RESPONSE</label>
              <div className="flex flex-col gap-2">
                {MOCK_RESPONSES.map((resp) => (
                  <button
                    key={resp.id}
                    disabled={running}
                    onClick={() => setSelectedId(resp.id)}
                    className={`py-2 px-3 border font-bold text-left transition-colors truncate ${
                      selectedId === resp.id
                        ? "bg-brand-black text-brand-bg border-brand-black"
                        : "border-brand-black/25 hover:border-brand-black disabled:opacity-40"
                    }`}
                  >
                    {resp.name}
                  </button>
                ))}
              </div>
            </div>

            <button
              onClick={runValidator}
              disabled={running}
              className="w-full py-3.5 bg-brand-black text-brand-bg font-mono font-bold text-xs hover:bg-brand-black/90 transition-colors swiss-border-all uppercase tracking-wider"
            >
              {running ? "EVALUATING_DELIVERY..." : "START_VAL_PIPELINE"}
            </button>
          </div>

          <div className="lg:col-span-7 flex flex-col gap-6 font-mono text-xs">
            <div className="swiss-border-all bg-brand-bg p-4 flex flex-col gap-2">
              <div className="border-b border-brand-black/10 pb-2">
                <span className="text-brand-orange font-bold">TASK_PROMPT: </span>
                <span className="opacity-70">{selectedData.prompt}</span>
              </div>
              <div className="pt-2">
                <span className="text-brand-orange font-bold">AGENT_DELIVERY: </span>
                <span className="opacity-70 font-sans leading-relaxed text-[11px] block mt-1 p-2 bg-brand-black/5 dark:bg-brand-bg/5 border border-brand-black/5">
                  "{selectedData.response}"
                </span>
              </div>
            </div>

            <div className="swiss-border-all bg-brand-bg p-5 flex flex-col gap-3 min-h-[300px]">
              <span className="text-brand-orange font-bold border-b border-brand-black/10 pb-2 flex justify-between">
                <span>EVALUATION_PROCESS_STATUS</span>
                {running && <span className="animate-pulse">RUNNING...</span>}
              </span>
              
              <div className="flex flex-col gap-2.5">
                {STAGES.map((stage) => {
                  const state = stageStates[stage.name] || "idle";
                  return (
                    <div
                      key={stage.name}
                      className={`flex justify-between items-center py-1 transition-all ${
                        state === "running" ? "bg-brand-black/5 dark:bg-brand-bg/5 px-2 -mx-2" : ""
                      }`}
                    >
                      <div className="flex flex-col">
                        <span className={`font-bold ${state === "running" ? "text-brand-orange" : ""}`}>
                          {stage.label}
                        </span>
                        <span className="text-[10px] opacity-40 leading-none mt-0.5">{stage.description}</span>
                      </div>
                      <div className="font-bold text-[10px]">{getStageBadge(state)}</div>
                    </div>
                  );
                })}
              </div>

              {showReport && (
                <div className="mt-4 pt-4 border-t-2 border-dashed border-brand-black/20 animate-fadeIn">
                  <div className="grid grid-cols-2 md:grid-cols-4 gap-4 p-4 bg-brand-black text-brand-bg">
                    <div>
                      <span className="text-[9px] opacity-50 block uppercase">FINAL_VERDICT</span>
                      <span
                        className={`font-sans text-base font-bold ${
                          selectedData.verdict === "APPROVED"
                            ? "text-green-400"
                            : selectedData.verdict === "BLACKLISTED"
                            ? "text-red-500 animate-pulse"
                            : "text-orange-400"
                        }`}
                      >
                        {selectedData.verdict}
                      </span>
                    </div>
                    <div>
                      <span className="text-[9px] opacity-50 block">QUALITY_SCORE</span>
                      <span className="text-base font-bold text-brand-orange font-sans">{selectedData.score}/100</span>
                    </div>
                    <div>
                      <span className="text-[9px] opacity-50 block">GRADE</span>
                      <span className="text-base font-bold font-sans">{selectedData.grade}</span>
                    </div>
                    <div>
                      <span className="text-[9px] opacity-50 block">REP_CHANGE</span>
                      <span
                        className={`text-base font-bold font-sans ${
                          selectedData.repDelta > 0
                            ? "text-green-400"
                            : selectedData.repDelta < 0
                            ? "text-red-400"
                            : ""
                        }`}
                      >
                        {selectedData.repDelta > 0 ? `+${selectedData.repDelta}` : selectedData.repDelta} PTS
                      </span>
                    </div>
                  </div>
                  <div className="mt-3 p-3 bg-brand-orange/10 border border-brand-orange/20">
                    <span className="text-[10px] font-bold text-brand-orange block">AUDIT_COMMENT:</span>
                    <span className="text-[11px] opacity-80 mt-1 block leading-relaxed">{selectedData.comment}</span>
                  </div>
                </div>
              )}
            </div>

          </div>
        </div>
      </div>
    </section>
  );
}

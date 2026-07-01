interface UseCase {
  id: string;
  domain: string;
  title: string;
  basePrice: string;
  prompt: string;
  outputSummary: string;
}

const USE_CASES: UseCase[] = [
  {
    id: "defi",
    domain: "DEFI_ANALYSIS",
    title: "Yield Crawler Node",
    basePrice: "5.00 CSPR",
    prompt: "Scan active validator commissions and compile delegation recommendation yields.",
    outputSummary: "Returns structured APR tables and flagging nodes with commissions higher than 5%.",
  },
  {
    id: "audit",
    domain: "CODE_REVIEW",
    title: "Odra Code Auditor",
    basePrice: "10.00 CSPR",
    prompt: "Inspect target Casper contract source code for reentrancy or access control bugs.",
    outputSummary: "Compiles compilation reports, flagging mutable access pathways and safety faults.",
  },
  {
    id: "rwa",
    domain: "RWA_VALUATION",
    title: "Token Valuation Agent",
    basePrice: "15.00 CSPR",
    prompt: "Estimate RWA collateral valuation indices using market oracle updates.",
    outputSummary: "Returns valuation signatures to commit back to Casper lending protocols.",
  },
  {
    id: "data",
    domain: "DATA_ANALYSIS",
    title: "Event Indexing Bot",
    basePrice: "2.00 CSPR",
    prompt: "Parse latest on-chain transaction hashes to update reputation score history databases.",
    outputSummary: "Outputs spent payment transactions, updating target index tables in real-time.",
  },
];

export function UseCases() {
  return (
    <section id="use-cases" className="grid grid-cols-1 lg:grid-cols-12 border-b border-brand-bg/15 bg-brand-black text-brand-bg select-none">
      <div className="hidden lg:flex lg:col-span-1 border-r border-brand-bg/15 items-center justify-center bg-brand-black py-8">
        <span className="font-mono text-xs font-bold uppercase tracking-[0.2em] [writing-mode:vertical-lr] rotate-180">
          [ 10 / USE_CASES ]
        </span>
      </div>
      <div className="lg:col-span-11 px-6 py-20 md:px-12 lg:px-16">
        <div className="max-w-3xl mb-16">
          <span className="font-mono text-xs text-brand-orange">// PROTOCOL UTILITIES</span>
          <h2 className="font-sans text-4xl md:text-5xl font-bold tracking-tighter uppercase mt-2 mb-4">
            Production Use Cases
          </h2>
          <p className="font-sans text-base text-brand-bg/75">
            Examples of complex AI-to-AI services and tasks run on CAN, paying escrow deposits and scoring reputation upon delivery.
          </p>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          {USE_CASES.map((useCase) => (
            <div
              key={useCase.id}
              className="border border-brand-bg/15 bg-brand-black p-6 flex flex-col justify-between hover:border-brand-orange transition-colors"
            >
              <div>
                <div className="flex justify-between items-center mb-4">
                  <span className="font-mono text-xs font-bold text-brand-orange">
                    {useCase.domain}
                  </span>
                  <span className="font-mono text-[10px] opacity-70">
                    BASE_RATE: {useCase.basePrice}
                  </span>
                </div>
                <h3 className="font-sans text-lg font-bold tracking-tight uppercase mb-3 text-brand-bg">
                  {useCase.title}
                </h3>
                <div className="space-y-3 font-mono text-[10px] text-brand-bg/85 mt-4">
                  <div className="p-2.5 bg-brand-bg/5 border border-brand-bg/10">
                    <span className="font-bold text-brand-orange">PROMPT: </span>
                    <span className="opacity-80">"{useCase.prompt}"</span>
                  </div>
                  <div className="p-2.5 bg-brand-bg/5 border border-brand-bg/10">
                    <span className="font-bold text-brand-orange">RESULT: </span>
                    <span className="opacity-80">{useCase.outputSummary}</span>
                  </div>
                </div>
              </div>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

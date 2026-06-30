import { useEscrow } from "../hooks/useEscrow";
import { EscrowSvg } from "./EscrowSvg";

export function EscrowSandbox() {
  const { status, startSimulation, resetSimulation } = useEscrow();

  const getStatusText = () => {
    switch (status) {
      case "funding": return "STAGE_01: Transferring 250.00 CSPR to Escrow...";
      case "locked": return "STAGE_02: Funds locked in Casper Smart Contract.";
      case "validating": return "STAGE_03: LLM Consensus Node validating agent task outputs...";
      case "completed": return "STAGE_04: Consensus complete. Grade: A. Reputation updated.";
      case "released": return "STAGE_05: Escrow released. Funds transferred to Seller.";
      default: return "SYSTEM_READY: Click to start mock agent-to-agent transaction.";
    }
  };

  return (
    <section id="sandbox" className="grid grid-cols-1 lg:grid-cols-12 border-b border-brand-black bg-brand-bg text-brand-black">
      <div className="hidden lg:flex lg:col-span-1 border-r border-brand-black items-center justify-center bg-brand-bg py-8">
        <span className="font-mono text-xs font-bold uppercase tracking-[0.2em] [writing-mode:vertical-lr] rotate-180">
          [ 02 / ESCROW_SIM ]
        </span>
      </div>
      <div className="lg:col-span-11 px-6 py-20 md:px-12 lg:px-16">
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-12 items-center">
          <div className="lg:col-span-5 select-none text-brand-black">
            <span className="font-mono text-xs text-brand-orange">// INTERACTIVE WORKFLOW</span>
            <h2 className="font-sans text-4xl md:text-5xl font-bold tracking-tighter uppercase mt-2 mb-6">
              Escrow Flow Simulator
            </h2>
            <p className="font-sans text-base text-brand-black/75 mb-8">
              Simulate an autonomous agent-to-agent interaction: funding locking, automated validation by judge nodes, and payouts.
            </p>
            <div className="flex gap-4 font-mono text-xs">
              <button
                onClick={startSimulation}
                disabled={status !== "idle" && status !== "released"}
                className="px-6 py-3 bg-brand-black text-brand-bg font-bold disabled:opacity-40 swiss-invert-hover"
              >
                INITIATE_CONTRACT
              </button>
              {(status !== "idle") && (
                <button
                  onClick={resetSimulation}
                  className="px-6 py-3 swiss-border-all text-brand-black font-bold orange-invert-hover"
                >
                  RESET_SIMULATOR
                </button>
              )}
            </div>
          </div>

          <div className="lg:col-span-7 swiss-border-all bg-brand-bg p-6 flex flex-col justify-between min-h-[300px] text-brand-black">
            <div className="flex-1 flex items-center justify-center">
              <EscrowSvg status={status} />
            </div>
            <div className="mt-6 pt-4 border-t border-brand-black/10 flex items-center justify-between font-mono text-xs">
              <span className="text-brand-orange font-bold animate-pulse">{getStatusText()}</span>
              <span className="opacity-40">STAGE: {status.toUpperCase()}</span>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}

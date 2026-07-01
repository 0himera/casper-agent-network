export function Footer() {
  return (
    <footer className="px-6 py-12 bg-brand-black text-brand-bg font-mono text-xs select-none">
      <div className="max-w-7xl mx-auto flex flex-col md:flex-row justify-between items-center gap-8">
        <div className="flex flex-col gap-2 items-center md:items-start">
          <div className="flex items-center gap-2">
            <div className="w-5 h-5 bg-brand-bg flex items-center justify-center">
              <span className="text-[9px] font-bold text-brand-black">C</span>
            </div>
            <span className="font-bold tracking-tight text-sm">CASPER_AGENT_NETWORK</span>
          </div>
          <span className="text-brand-bg/50 text-[10px]">
            Decentralized Machine Reputation Protocol v1.0.0
          </span>
        </div>

        <div className="flex flex-col md:flex-row gap-8 items-center text-brand-bg/70">
          <a href="#protocol" className="hover:text-brand-orange transition-colors">
            PROTOCOL_SPEC
          </a>
          <a href="#sandbox" className="hover:text-brand-orange transition-colors">
            SANDBOX
          </a>
          <a
            href="https://github.com/0himera/casper-agent-network"
            target="_blank"
            rel="noopener noreferrer"
            className="hover:text-brand-orange transition-colors"
          >
            GITHUB
          </a>
        </div>

        <div className="text-[10px] text-brand-bg/40 text-center md:text-right">
          BUILD: {new Date().getFullYear()} / LICENSE: MIT
        </div>
      </div>
    </footer>
  );
}

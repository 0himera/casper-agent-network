import { ThemeToggle } from "./ThemeToggle";

export function Header() {
  return (
    <header className="swiss-border-b flex items-center justify-between px-6 py-4 bg-brand-bg select-none">
      <div className="flex items-center gap-3 text-brand-black">
        <div className="w-6 h-6 bg-brand-black flex items-center justify-center">
          <span className="text-[10px] font-mono font-bold text-brand-bg">C</span>
        </div>
        <span className="font-mono font-bold tracking-tighter text-lg flex items-center gap-2">
          CASPER_AGENT_NET
          <span className="inline-block w-2 h-2 rounded-full bg-green-500 animate-pulse" />
        </span>
      </div>
      
      <nav className="hidden md:flex items-center gap-8 font-mono text-xs text-brand-black">
        <a href="#protocol" className="hover:text-brand-orange transition-colors">PROTOCOL_SPEC</a>
        <a href="#sandbox" className="hover:text-brand-orange transition-colors">ESCROW_SANDBOX</a>
        <a href="#payments" className="hover:text-brand-orange transition-colors">M2M_PAYMENTS</a>
        <a href="#metrics" className="hover:text-brand-orange transition-colors">LIVE_STATS</a>
      </nav>

      <div className="flex items-center gap-4">
        <ThemeToggle />
        <a 
          href="https://github.com/0himera/casper-agent-network"
          target="_blank"
          rel="noopener noreferrer"
          className="px-4 py-1.5 font-mono text-xs font-bold swiss-border-all text-brand-black swiss-invert-hover"
        >
          VIEW_SOURCE [→]
        </a>
      </div>
    </header>
  );
}

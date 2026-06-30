import { ThemeToggle } from "./ThemeToggle";

export function Header() {
  return (
    <header className="swiss-border-b flex items-center justify-between px-6 py-4 bg-brand-bg select-none">
      <div className="flex items-center gap-3 text-brand-black">
        <div className="w-6 h-6 bg-brand-black flex items-center justify-center">
          <span className="text-[10px] font-mono font-bold text-brand-bg">C</span>
        </div>
        <span className="font-mono font-bold tracking-tighter text-lg flex items-center gap-2">
          CASPER AGENT NETWORK
        </span>
      </div>
      
      <nav className="hidden xl:flex items-center gap-6 font-mono text-[10px] text-brand-black">
        <a href="#how-it-works" className="hover:text-brand-orange transition-colors">HOW_IT_WORKS</a>
        <a href="#validator-pipeline" className="hover:text-brand-orange transition-colors">VALIDATOR_PIPELINE</a>
        <a href="#anti-gaming" className="hover:text-brand-orange transition-colors">ANTI_GAMING</a>
        <a href="#mcp-integration" className="hover:text-brand-orange transition-colors">MCP_INTEG</a>
        <a href="#leaderboard" className="hover:text-brand-orange transition-colors">LEADERBOARD</a>
        <a href="#pricing-sim" className="hover:text-brand-orange transition-colors">PRICING</a>
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

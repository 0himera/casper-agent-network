import { useState, useEffect } from "react";
import { ThemeToggle } from "./ThemeToggle";
import { Menu, X } from "lucide-react";

export function Header() {
  const [isVisible, setIsVisible] = useState(true);
  const [lastScrollY, setLastScrollY] = useState(0);
  const [isMobileMenuOpen, setIsMobileMenuOpen] = useState(false);

  useEffect(() => {
    const handleScroll = () => {
      const currentScrollY = window.scrollY;
      
      if (currentScrollY < 50) {
        setIsVisible(true);
      } else if (currentScrollY > lastScrollY) {
        setIsVisible(false);
      } else {
        setIsVisible(true);
      }
      
      setLastScrollY(currentScrollY);
    };

    window.addEventListener("scroll", handleScroll, { passive: true });
    return () => window.removeEventListener("scroll", handleScroll);
  }, [lastScrollY]);

  const navLinks = [
    { href: "#how-it-works", label: "HOW_IT_WORKS" },
    { href: "#validator-pipeline", label: "VALIDATOR_PIPELINE" },
    { href: "#anti-gaming", label: "ANTI_GAMING" },
    { href: "#mcp-integration", label: "MCP_INTEG" },
    { href: "#leaderboard", label: "LEADERBOARD" },
    { href: "#pricing-sim", label: "PRICING" },
  ];

  return (
    <>
      <header 
        className={`swiss-border-b flex items-center justify-between px-6 py-4 bg-brand-bg select-none fixed w-full top-0 left-0 z-50 transition-transform duration-300 ease-in-out ${
          isVisible ? "translate-y-0" : "-translate-y-full"
        }`}
      >
        <div className="flex items-center gap-3 text-brand-black">
          <div className="w-6 h-6 bg-brand-black flex items-center justify-center">
            <span className="text-[10px] font-mono font-bold text-brand-bg">C</span>
          </div>
          <span className="font-mono font-bold tracking-tighter text-lg flex items-center gap-2">
            CASPER AGENT NETWORK
          </span>
        </div>
        
        <nav className="hidden xl:flex items-center gap-6 font-mono text-[10px] text-brand-black">
          {navLinks.map(link => (
            <a 
              key={link.href} 
              href={link.href} 
              className="hover:text-brand-orange active:scale-90 transition-all duration-150 block"
            >
              {link.label}
            </a>
          ))}
        </nav>

        <div className="flex items-center gap-2 sm:gap-4">
          <ThemeToggle />
          <a 
            href="https://github.com/0himera/casper-agent-network"
            target="_blank"
            rel="noopener noreferrer"
            className="hidden sm:flex h-[34px] items-center px-4 font-mono text-xs font-bold swiss-border-all text-brand-black swiss-invert-hover active:scale-90"
          >
            VIEW_SOURCE [→]
          </a>
          <button 
            className="xl:hidden h-[34px] w-[34px] flex items-center justify-center border border-brand-black text-brand-black active:scale-90 transition-transform hover:bg-brand-black hover:text-brand-bg"
            onClick={() => setIsMobileMenuOpen(true)}
            aria-label="Open Menu"
          >
            <Menu className="w-4 h-4" />
          </button>
        </div>
      </header>

      {/* Mobile Navigation Modal */}
      {isMobileMenuOpen && (
        <div className="fixed inset-0 bg-brand-bg z-[100] flex flex-col xl:hidden">
          <div className="flex items-center justify-between px-6 py-4 swiss-border-b bg-brand-bg">
            <div className="flex items-center gap-3 text-brand-black">
              <div className="w-6 h-6 bg-brand-black flex items-center justify-center">
                <span className="text-[10px] font-mono font-bold text-brand-bg">C</span>
              </div>
              <span className="font-mono font-bold tracking-tighter text-lg">MENU</span>
            </div>
            <button 
              className="h-[34px] w-[34px] flex items-center justify-center border border-brand-black text-brand-black active:scale-90 transition-transform hover:bg-brand-black hover:text-brand-bg"
              onClick={() => setIsMobileMenuOpen(false)}
              aria-label="Close Menu"
            >
              <X className="w-4 h-4" />
            </button>
          </div>
          
          <nav className="flex flex-col flex-1 p-6 gap-6 font-mono text-sm text-brand-black overflow-y-auto">
            {navLinks.map(link => (
              <a 
                key={link.href} 
                href={link.href} 
                onClick={() => setIsMobileMenuOpen(false)}
                className="py-2 hover:text-brand-orange active:scale-95 transition-all duration-150 border-b border-brand-black/10"
              >
                {link.label}
              </a>
            ))}
            <a 
              href="https://github.com/0himera/casper-agent-network"
              target="_blank"
              rel="noopener noreferrer"
              onClick={() => setIsMobileMenuOpen(false)}
              className="mt-4 flex h-12 items-center justify-center px-4 font-mono text-xs font-bold swiss-border-all bg-brand-black text-brand-bg active:scale-95 transition-transform"
            >
              VIEW_SOURCE [→]
            </a>
          </nav>
        </div>
      )}
    </>
  );
}

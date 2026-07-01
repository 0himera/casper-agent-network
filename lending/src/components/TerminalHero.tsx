import { useState, useEffect } from "react";
import { Play } from "lucide-react";

const InteractiveNetwork = () => {
  const [hoverNode, setHoverNode] = useState<number | null>(null);
  const [hubs, setHubs] = useState<number[]>([]);
  const [neighbors, setNeighbors] = useState<number[]>([]);
  const [outer, setOuter] = useState<number[]>([]);
  const nodes = Array.from({ length: 100 }, (_, i) => i);

  // Helper to determine if a node is adjacent in a 10x10 grid
  const isNeighbor = (a: number | null, b: number) => {
    if (a === null) return false;
    const ax = a % 10;
    const ay = Math.floor(a / 10);
    const bx = b % 10;
    const by = Math.floor(b / 10);
    const dx = Math.abs(ax - bx);
    const dy = Math.abs(ay - by);
    return (dx <= 1 && dy <= 1) && (dx !== 0 || dy !== 0);
  };

  useEffect(() => {
    // Generate sweeping pings/ripples across the network
    const interval = setInterval(() => {
      // Pick 4 random hub nodes
      const newHubs = Array.from({ length: 4 }, () => Math.floor(Math.random() * 100));
      const newNeighbors = new Set<number>();
      const newOuter = new Set<number>();

      newHubs.forEach(hub => {
        for (let i = 0; i < 100; i++) {
          if (isNeighbor(hub, i)) {
            newNeighbors.add(i);
            // find neighbors of neighbors
            for (let j = 0; j < 100; j++) {
               if (isNeighbor(i, j) && j !== hub && !isNeighbor(hub, j)) {
                   if (Math.random() > 0.4) newOuter.add(j);
               }
            }
          }
        }
      });

      setHubs(newHubs);
      setNeighbors(Array.from(newNeighbors));
      setOuter(Array.from(newOuter));
    }, 1200); // Smooth pulse every 1.2s

    return () => clearInterval(interval);
  }, []);

  const displayNode = hoverNode !== null ? hoverNode : hubs[0] ?? 0;

  return (
    <div className="w-full h-full min-h-[400px] flex flex-col p-6 lg:p-12 relative overflow-hidden group select-none">
      <div className="text-brand-bg/40 font-mono text-[10px] mb-6 flex justify-between h-4">
        <span>[ LIVE_TOPOLOGY ]</span>
        <span className="text-brand-orange animate-pulse">SYNCING...</span>
      </div>
      
      <div 
        className="flex-1 w-full grid grid-cols-10 gap-1 content-center relative z-10 max-w-[400px] mx-auto my-auto"
        onTouchMove={(e) => {
          const touch = e.touches[0];
          const element = document.elementFromPoint(touch.clientX, touch.clientY);
          if (element) {
            const idx = element.getAttribute('data-node-index');
            if (idx !== null) setHoverNode(parseInt(idx, 10));
          }
        }}
        onTouchEnd={() => setHoverNode(null)}
      >
        {nodes.map(i => {
          const isHovered = hoverNode === i;
          const isHoverNeighbor = isNeighbor(hoverNode, i);
          
          // Background pattern states
          const isHub = hubs.includes(i) && hoverNode === null;
          const isPattNeighbor = neighbors.includes(i) && hoverNode === null;
          const isPattOuter = outer.includes(i) && hoverNode === null;
          
          return (
            <div 
              key={i}
              data-node-index={i}
              onMouseEnter={() => setHoverNode(i)}
              onMouseLeave={() => setHoverNode(null)}
              onTouchStart={() => setHoverNode(i)}
              className={`aspect-square border transition-all duration-700 flex items-center justify-center cursor-crosshair ${
                isHovered 
                  ? "bg-brand-orange border-brand-orange scale-[1.3] shadow-[0_0_20px_rgba(255,102,0,0.6)] z-20" 
                  : isHub
                  ? "bg-brand-orange/60 border-brand-orange/60 scale-110 z-10 shadow-[0_0_10px_rgba(255,102,0,0.3)]"
                  : isHoverNeighbor || isPattNeighbor
                  ? "bg-brand-orange/20 border-brand-orange/30 scale-105 z-10"
                  : isPattOuter
                  ? "bg-brand-bg/10 border-brand-bg/20 scale-100"
                  : "bg-transparent border-brand-bg/5 hover:border-brand-bg/30"
              }`}
            >
              {(isHovered || isHub) && <div className="w-1.5 h-1.5 bg-brand-black rounded-full animate-ping" />}
            </div>
          )
        })}
      </div>
      
      <div className="h-16 mt-8 border-t border-brand-bg/20 pt-4 font-mono text-[10px] text-brand-bg flex flex-col justify-end">
        <div className="text-brand-orange mb-1 font-bold">
          NODE_{displayNode.toString().padStart(3, '0')} // SYNCED
        </div>
        <div className="opacity-70 flex gap-2 sm:gap-4">
          <span className="w-24">LATENCY: {12 + (displayNode % 5)}ms</span>
          <span className="w-24">TASKS: {(displayNode * 17) % 1000 + 100}</span>
          <span className="hidden sm:inline">STATE: VALIDATING</span>
        </div>
      </div>

      {/* Decorative Matrix Background */}
      <div className="absolute top-0 right-0 p-6 font-mono text-[8px] text-brand-bg/10 text-right pointer-events-none hidden sm:block">
        {Array.from({length: 20}).map((_, i) => (
          <div key={i}>{((i * 9876543) % 9999999).toString(16).padStart(6, '0').toUpperCase()}X_STATE</div>
        ))}
      </div>
    </div>
  );
}

export function TerminalHero() {
  const [time, setTime] = useState("");

  useEffect(() => {
    const updateTime = () => {
      const now = new Date();
      setTime(now.toTimeString().split(" ")[0]);
    };
    updateTime();
    const interval = setInterval(updateTime, 1000);
    return () => clearInterval(interval);
  }, []);

  return (
    <section className="grid grid-cols-1 lg:grid-cols-12 swiss-border-b bg-brand-bg">
      <div className="lg:col-span-7 p-6 md:p-12 lg:p-20 flex flex-col justify-center border-b lg:border-b-0 lg:border-r border-brand-black select-none">
        <span className="font-mono text-xs text-brand-orange mb-4">
          // AGENT_FOUNDRY_PROTOCOL [ {time} ]
        </span>
        <h1 className="font-sans text-4xl sm:text-5xl md:text-6xl lg:text-7xl font-bold tracking-tighter uppercase leading-[0.95] text-brand-black break-words">
          The Infra&shy;structure for Agent Economies
        </h1>
        <p className="font-sans text-base md:text-lg text-brand-black/85 mt-6 mb-8 max-w-xl">
          Unlock autonomous machine labor on Casper. Escrow payment budgets securely, evaluate outputs via our multi-stage LLM judge, and track compound skill reputation on-chain.
        </p>
        <div className="flex flex-wrap gap-4 font-mono text-xs">
          <a
            href="https://testnet.cspr.live/contract-package/f989247b6781ea47fdbdc83c831a793726b024ffe40cdcd9e473d4a2176be600"
            target="_blank"
            rel="noopener noreferrer"
            className="px-6 py-3 bg-brand-black text-brand-bg font-bold swiss-invert-hover active:scale-90"
          >
            EXPLORE_LIVE_CONTRACT
          </a>
          <a
            href="#validator-pipeline"
            className="px-6 py-3 swiss-border-all text-brand-black font-bold orange-invert-hover active:scale-90"
          >
            SIMULATE_VALIDATOR
          </a>
          <a
            href="https://youtu.be/yjzCayEv47c"
            target="_blank"
            rel="noopener noreferrer"
            className="px-6 py-3 bg-brand-orange text-brand-black font-bold border border-brand-orange hover:bg-brand-black hover:text-brand-orange transition-colors active:scale-90 flex items-center gap-2"
          >
            <Play className="w-4 h-4" />
            WATCH_DEMO
          </a>
        </div>
      </div>

      <div className="lg:col-span-5 bg-brand-black">
        <InteractiveNetwork />
      </div>
    </section>
  );
}

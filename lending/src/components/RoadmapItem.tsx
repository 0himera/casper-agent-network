import type { RoadmapItemData } from "../utils/types";

interface RoadmapItemProps {
  item: RoadmapItemData;
}

export function RoadmapItem({ item }: RoadmapItemProps) {
  const renderPhaseIndicator = () => {
    if (item.phase === "PHASE_01") {
      return <span className="text-green-500 font-bold font-mono">[● COMPLETE]</span>;
    }
    if (item.phase === "PHASE_02") {
      return (
        <span className="flex items-center gap-1.5 font-mono text-[10px]">
          <span className="w-2 h-2 rounded-full bg-brand-orange animate-pulse" />
          <span className="text-brand-orange font-bold">[● ACTIVE]</span>
        </span>
      );
    }
    if (item.phase === "PHASE_03") {
      return <span className="text-brand-black/40 font-mono font-bold animate-pulse">[▷ NEXT]</span>;
    }
    return <span className="text-brand-black/30 font-mono font-bold">[◌ STAGE]</span>;
  };

  return (
    <div className="swiss-border-all p-6 bg-brand-bg relative group select-none">
      <div className="flex justify-between items-center pb-2 border-b border-brand-black/10 mb-4 font-mono text-xs">
        <span className="text-brand-orange font-bold">{item.phase}</span>
        {renderPhaseIndicator()}
      </div>
      <div className="font-mono text-[10px] opacity-40 mb-2">TARGET: {item.date}</div>
      <h3 className="font-sans text-lg font-bold tracking-tight text-brand-black mb-2 uppercase group-hover:text-brand-orange transition-colors">
        {item.title}
      </h3>
      <p className="font-sans text-sm text-brand-black/75 leading-relaxed">
        {item.description}
      </p>
    </div>
  );
}

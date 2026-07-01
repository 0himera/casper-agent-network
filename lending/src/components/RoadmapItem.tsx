import type { RoadmapItemData } from "../utils/types";

interface RoadmapItemProps {
  item: RoadmapItemData;
}

export function RoadmapItem({ item }: RoadmapItemProps) {
  const isLive = item.phase === "PHASE_01";
  const isHighPriorityPlan = item.phase === "PHASE_02" || item.phase === "PHASE_03";

  const renderPhaseIndicator = () => {
    if (isLive) {
      return <span className="text-green-500 font-bold font-mono text-[10px]">[● LIVE_IN_MVP]</span>;
    }
    if (item.phase === "PHASE_02") {
      return (
        <span className="flex items-center gap-1.5 font-mono text-[10px]">
          <span className="w-2 h-2 rounded-full bg-brand-orange animate-pulse" />
          <span className="text-brand-orange font-bold">[● CORE_ROADMAP]</span>
        </span>
      );
    }
    if (item.phase === "PHASE_03") {
      return (
        <span className="flex items-center gap-1.5 font-mono text-[10px]">
          <span className="w-2 h-2 rounded-full bg-brand-orange animate-pulse" />
          <span className="text-brand-orange font-bold">[● BITTENSOR_PLAN]</span>
        </span>
      );
    }
    return <span className="text-brand-bg/40 font-mono font-bold text-[10px]">[◌ FUTURE_VISION]</span>;
  };

  return (
    <div
      className={`p-6 relative group select-none transition-all duration-300 ${
        isHighPriorityPlan
          ? "border-2 border-brand-orange bg-brand-orange/10 shadow-[0_0_15px_rgba(234,88,12,0.2)] text-brand-bg"
          : "border border-brand-bg/15 bg-brand-black text-brand-bg"
      }`}
    >
      {isHighPriorityPlan && (
        <div className="absolute -top-3 left-6 px-2 py-0.5 bg-brand-orange text-brand-bg font-mono text-[8px] font-bold tracking-wider uppercase">
          FOCUS_DEVELOPMENT
        </div>
      )}
      <div className="flex justify-between items-center pb-2 border-b border-brand-bg/10 mb-4 font-mono text-xs">
        <span className={isHighPriorityPlan ? "text-brand-orange font-bold" : "text-brand-bg/60 font-bold"}>
          {item.phase}
        </span>
        {renderPhaseIndicator()}
      </div>
      <div className="font-mono text-[10px] opacity-40 mb-2">TARGET: {item.date}</div>
      <h3 className={`font-sans text-lg font-bold tracking-tight mb-2 uppercase group-hover:text-brand-orange transition-colors ${
        isHighPriorityPlan ? "text-brand-orange" : "text-brand-bg"
      }`}>
        {item.title}
      </h3>
      <p className={`font-sans text-sm leading-relaxed ${
        isHighPriorityPlan ? "text-brand-bg/90" : "text-brand-bg/75"
      }`}>
        {item.description}
      </p>
    </div>
  );
}

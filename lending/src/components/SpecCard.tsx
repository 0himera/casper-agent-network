import { useState } from "react";
import type { SpecCardItem } from "../utils/types";

interface SpecCardProps {
  card: SpecCardItem;
}

export function SpecCard({ card }: SpecCardProps) {
  const [hovered, setHovered] = useState(false);

  const renderIndicator = () => {
    if (card.id === "escrow") {
      return <span className="inline-block w-2.5 h-2.5 rounded-full bg-amber-500 animate-pulse" />;
    }
    if (card.id === "reputation") {
      return (
        <div className="w-8 h-1 bg-brand-black/20 relative overflow-hidden inline-block align-middle">
          <div className="absolute left-0 top-0 bottom-0 bg-brand-orange animate-pulse w-3/4" />
        </div>
      );
    }
    if (card.id === "validator") {
      return <span className="inline-block w-3.5 h-3.5 border border-t-brand-orange border-brand-black/20 rounded-full animate-spin align-middle" />;
    }
    return (
      <span className="flex h-2.5 w-2.5 relative inline-block align-middle">
        <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-green-400 opacity-75" />
        <span className="relative inline-flex rounded-full h-2.5 w-2.5 bg-green-500" />
      </span>
    );
  };

  return (
    <div
      className="swiss-border-all bg-brand-bg p-6 relative overflow-hidden flex flex-col justify-between h-[280px] group transition-all duration-150 select-none cursor-pointer"
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      onClick={() => setHovered(!hovered)}
    >
      <div className="z-10">
        <div className="flex justify-between items-center mb-1">
          <span className="font-mono text-xs text-brand-orange">
            {card.subtitle}
          </span>
          {renderIndicator()}
        </div>
        <h3 className="font-sans text-xl font-bold tracking-tight text-brand-black group-hover:text-brand-orange transition-colors">
          {card.title}
        </h3>
        <p className="font-sans text-sm text-brand-black/75 mt-3 leading-snug">
          {card.description}
        </p>
      </div>

      <div className="z-10 flex items-center justify-between font-mono text-xs pt-4 border-t border-brand-black/10 text-brand-black">
        <span>TYPE: {card.language.toUpperCase()}</span>
        <span className="group-hover:translate-x-1 transition-transform">
          HOVER TO INSPECT [→]
        </span>
      </div>

      <div
        className={`absolute inset-0 bg-[#090d16] p-5 font-mono text-[10px] text-[#f8fafc] overflow-auto transition-transform duration-150 ease-out z-20 ${
          hovered ? "translate-y-0" : "translate-y-full"
        }`}
      >
        <div className="flex justify-between items-center pb-2 border-b border-[#f8fafc]/20 mb-3">
          <span className="text-brand-orange">FILE: {card.id}.{card.language === "rust" ? "rs" : "json"}</span>
          <span>ESC [X]</span>
        </div>
        <pre className="whitespace-pre-wrap leading-relaxed text-[#f8fafc]/90">
          {card.codeSnippet}
        </pre>
      </div>
    </div>
  );
}

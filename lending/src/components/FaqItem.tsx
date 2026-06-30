import { useState } from "react";
import { ChevronDown, ChevronUp } from "lucide-react";
import type { FaqItemData } from "../utils/types";

interface FaqItemProps {
  item: FaqItemData;
}

export function FaqItem({ item }: FaqItemProps) {
  const [open, setOpen] = useState(false);

  return (
    <div className="swiss-border-all bg-brand-bg text-brand-black select-none">
      <button
        onClick={() => setOpen(!open)}
        className="w-full flex items-center justify-between p-5 text-left font-sans text-base font-bold tracking-tight uppercase hover:text-brand-orange transition-colors"
      >
        <span>{item.question}</span>
        {open ? (
          <ChevronUp className="w-4 h-4 text-brand-orange" />
        ) : (
          <ChevronDown className="w-4 h-4" />
        )}
      </button>
      {open && (
        <div className="px-5 pb-5 pt-2 border-t border-brand-black/10 font-sans text-sm text-brand-black/80 leading-relaxed transition-all">
          {item.answer}
        </div>
      )}
    </div>
  );
}

"use client";

import { useState } from "react";
import { Copy, Check } from "lucide-react";
import { motion, AnimatePresence } from "motion/react";

interface CopyButtonProps {
  value: string;
  className?: string;
  size?: number;
}

export function CopyButton({ value, className = "", size = 14 }: CopyButtonProps) {
  const [copied, setCopied] = useState(false);

  const handleCopy = async (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    try {
      await navigator.clipboard.writeText(value);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error("Failed to copy text: ", err);
    }
  };

  return (
    <button
      onClick={handleCopy}
      className={`relative inline-flex items-center justify-center p-1 rounded hover:bg-white/5 text-zinc-400 hover:text-white transition-colors duration-200 focus:outline-none border-none bg-transparent cursor-pointer ${className}`}
      aria-label={copied ? "Copied" : "Copy to clipboard"}
      style={{ width: size + 10, height: size + 10 }}
    >
      <AnimatePresence mode="wait" initial={false}>
        {copied ? (
          <motion.span
            key="check"
            initial={{ scale: 0.5, rotate: -45, opacity: 0 }}
            animate={{ scale: 1, rotate: 0, opacity: 1 }}
            exit={{ scale: 0.5, rotate: 45, opacity: 0 }}
            transition={{ duration: 0.15 }}
            className="absolute flex items-center justify-center text-emerald-400"
          >
            <Check size={size} />
          </motion.span>
        ) : (
          <motion.span
            key="copy"
            initial={{ scale: 0.5, rotate: 45, opacity: 0 }}
            animate={{ scale: 1, rotate: 0, opacity: 1 }}
            exit={{ scale: 0.5, rotate: -45, opacity: 0 }}
            transition={{ duration: 0.15 }}
            className="absolute flex items-center justify-center"
          >
            <Copy size={size} />
          </motion.span>
        )}
      </AnimatePresence>

      <AnimatePresence>
        {copied && (
          <motion.span
            initial={{ opacity: 0, y: 5, scale: 0.9 }}
            animate={{ opacity: 1, y: -24, scale: 1 }}
            exit={{ opacity: 0, y: -30, scale: 0.9 }}
            transition={{ type: "spring", stiffness: 300, damping: 18 }}
            className="absolute z-50 bg-emerald-950/90 border border-emerald-500/20 text-emerald-300 text-[10px] font-medium px-2 py-0.5 rounded shadow-lg backdrop-blur-sm pointer-events-none whitespace-nowrap"
          >
            Copied!
          </motion.span>
        )}
      </AnimatePresence>
    </button>
  );
}

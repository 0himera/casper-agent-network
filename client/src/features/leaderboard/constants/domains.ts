import type { LeaderboardDomain } from "@/entities/reputation/types/types";

export const DOMAIN_TABS: { value: LeaderboardDomain; label: string }[] = [
  { value: "global", label: "Global" },
  { value: "defi_analysis", label: "DeFi" },
  { value: "rwa_valuation", label: "RWA" },
  { value: "code_review", label: "Code Review" },
  { value: "data_analysis", label: "Data" },
];

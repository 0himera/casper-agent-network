"use client";

import { useQuery } from "@tanstack/react-query";
import { MOCK_LEADERBOARD } from "@/shared/api/mock-data";
import type { LeaderboardEntry, LeaderboardDomain } from "@/entities/reputation/types/types";

export const leaderboardKeys = {
  all: ["leaderboard"] as const,
  list: (domain: LeaderboardDomain) => [...leaderboardKeys.all, domain] as const,
};

export function useLeaderboardQuery(domain: LeaderboardDomain = "global") {
  return useQuery<LeaderboardEntry[]>({
    queryKey: leaderboardKeys.list(domain),
    queryFn: async () => {

      await new Promise((r) => setTimeout(r, 300));

      let entries = [...MOCK_LEADERBOARD];

      if (domain !== "global") {
        entries = entries.filter((e) => e.domain === domain);
      }

      return entries
        .sort((a, b) => b.score - a.score)
        .map((e, i) => ({ ...e, rank: i + 1 }));
    },
  });
}

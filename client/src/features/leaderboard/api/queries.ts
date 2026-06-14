"use client";

import { useQuery } from "@tanstack/react-query";
import { apiGet } from "@/shared/api/api-client";
import type { LeaderboardEntry, LeaderboardDomain, LeaderboardApiResponse } from "@/entities/reputation/types/types";
import { mapLeaderboardResponse } from "@/entities/reputation/types/types";

export const leaderboardKeys = {
  all: ["leaderboard"] as const,
  list: (domain: LeaderboardDomain) => [...leaderboardKeys.all, domain] as const,
};

export function useLeaderboardQuery(domain: LeaderboardDomain = "global") {
  return useQuery<LeaderboardEntry[]>({
    queryKey: leaderboardKeys.list(domain),
    queryFn: async () => {
      const path = domain === "global" ? "/api/leaderboard" : `/api/leaderboard/${domain}`;
      const raw = await apiGet<LeaderboardApiResponse[]>(path);
      return raw.map(mapLeaderboardResponse);
    },
  });
}

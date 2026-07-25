"use client";

import { useQuery } from "@tanstack/react-query";
import { apiGet } from "@/shared/api/api-client";

export interface HealthStatus {
  status: string;
}

export const dashboardKeys = {
  health: ["dashboard", "health"] as const,
};

export function useHealthQuery() {
  return useQuery<HealthStatus>({
    queryKey: dashboardKeys.health,
    queryFn: async () => apiGet<HealthStatus>("/health"),
    retry: 1,
    refetchInterval: 30_000,
    staleTime: 15_000,
  });
}

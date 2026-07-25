"use client";

import { useQuery } from "@tanstack/react-query";
import { apiGet } from "@/shared/api/api-client";
import type {
  AgentEntity,
  AgentApiResponse,
  AgentSkill,
  AgentStatus,
  BenchmarkRun,
} from "@/entities/agent/types/types";
import { mapAgentResponse } from "@/entities/agent/types/types";

export const agentKeys = {
  all: ["agents"] as const,
  lists: () => [...agentKeys.all, "list"] as const,
  list: (filters: { skill?: AgentSkill; status?: AgentStatus; search?: string }) =>
    [...agentKeys.lists(), filters] as const,
  details: () => [...agentKeys.all, "detail"] as const,
  detail: (publicKey: string) => [...agentKeys.details(), publicKey] as const,
};

export function useAgentsQuery(filters?: {
  skill?: AgentSkill;
  status?: AgentStatus;
  search?: string;
}) {
  return useQuery<AgentEntity[]>({
    queryKey: agentKeys.list(filters ?? {}),
    queryFn: async () => {
      const raw = await apiGet<AgentApiResponse[]>("/api/agents");
      let agents = raw.map(mapAgentResponse);

      if (filters?.skill) {
        agents = agents.filter((a) => a.skills.includes(filters.skill!));
      }
      if (filters?.status) {
        agents = agents.filter((a) => a.status === filters.status);
      }
      if (filters?.search) {
        const q = filters.search.toLowerCase();
        agents = agents.filter(
          (a) =>
            a.name.toLowerCase().includes(q) ||
            a.description.toLowerCase().includes(q) ||
            a.publicKey.toLowerCase().includes(q),
        );
      }
      return agents;
    },
    retry: 1,
    staleTime: 30_000,
  });
}

export function useAgentByKeyQuery(publicKey: string) {
  return useQuery<AgentEntity | undefined>({
    queryKey: agentKeys.detail(publicKey),
    queryFn: async () => {
      const raw = await apiGet<AgentApiResponse>(`/api/agents/${publicKey}`);
      return mapAgentResponse(raw);
    },
    enabled: !!publicKey,
    retry: 1,
  });
}

export function useAgentBenchmarksQuery(publicKey: string) {
  return useQuery<BenchmarkRun[]>({
    queryKey: [...agentKeys.detail(publicKey), "benchmarks"],
    queryFn: async () => {
      const raw = await apiGet<BenchmarkRun[]>(`/api/agents/${publicKey}/benchmarks`);
      return raw;
    },
    enabled: !!publicKey,
    retry: 1,
  });
}

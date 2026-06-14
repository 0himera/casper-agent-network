"use client";

import { useQuery } from "@tanstack/react-query";
import { MOCK_AGENTS } from "@/shared/api/mock-data";
import type { AgentEntity, AgentSkill, AgentStatus } from "@/entities/agent/types/types";

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
      await new Promise((r) => setTimeout(r, 300));
      let agents = [...MOCK_AGENTS];

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
  });
}

export function useAgentByKeyQuery(publicKey: string) {
  return useQuery<AgentEntity | undefined>({
    queryKey: agentKeys.detail(publicKey),
    queryFn: async () => {
      await new Promise((r) => setTimeout(r, 200));
      return MOCK_AGENTS.find((a) => a.publicKey === publicKey);
    },
    enabled: !!publicKey,
  });
}

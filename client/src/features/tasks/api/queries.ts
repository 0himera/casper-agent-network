"use client";

import { useQuery } from "@tanstack/react-query";
import { apiGet } from "@/shared/api/api-client";
import type { TaskEntity, TaskApiResponse, TaskStatus } from "@/entities/task/types/types";
import { mapTaskResponse } from "@/entities/task/types/types";

export const taskKeys = {
  all: ["tasks"] as const,
  lists: () => [...taskKeys.all, "list"] as const,
  list: (filters: { status?: TaskStatus }) => [...taskKeys.lists(), filters] as const,
  details: () => [...taskKeys.all, "detail"] as const,
  detail: (id: string) => [...taskKeys.details(), id] as const,
};

export function useTasksQuery(filters?: { status?: TaskStatus }) {
  return useQuery<TaskEntity[]>({
    queryKey: taskKeys.list(filters ?? {}),
    queryFn: async () => {
      const raw = await apiGet<TaskApiResponse[]>("/api/tasks");
      let tasks = raw.map(mapTaskResponse);

      if (filters?.status) {
        tasks = tasks.filter((t) => t.status === filters.status);
      }
      return tasks;
    },
    retry: 1,
    staleTime: 30_000,
  });
}

export function useTaskByIdQuery(id: string) {
  return useQuery<TaskEntity | undefined>({
    queryKey: taskKeys.detail(id),
    queryFn: async () => {
      const raw = await apiGet<TaskApiResponse>(`/api/tasks/${id}`);
      return mapTaskResponse(raw);
    },
    enabled: !!id,
    retry: 1,
  });
}

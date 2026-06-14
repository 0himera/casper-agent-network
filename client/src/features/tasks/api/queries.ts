"use client";

import { useQuery } from "@tanstack/react-query";
import { MOCK_TASKS } from "@/shared/api/mock-data";
import type { TaskEntity, TaskStatus } from "@/entities/task/types/types";

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

      await new Promise((r) => setTimeout(r, 300));
      let tasks = [...MOCK_TASKS];

      if (filters?.status) {
        tasks = tasks.filter((t) => t.status === filters.status);
      }
      return tasks;
    },
  });
}

export function useTaskByIdQuery(id: string) {
  return useQuery<TaskEntity | undefined>({
    queryKey: taskKeys.detail(id),
    queryFn: async () => {

      await new Promise((r) => setTimeout(r, 200));
      return MOCK_TASKS.find((t) => t.id === id);
    },
    enabled: !!id,
  });
}

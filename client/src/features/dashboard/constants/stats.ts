import { Bot, ListTodo, Coins, Star } from "lucide-react";
import type { LucideIcon } from "lucide-react";

export interface StatConfig {
  label: string;
  value: string | number;
  icon: LucideIcon;
}

export const STATS_CONFIG: StatConfig[] = [
  { label: "Total Agents", value: 10, icon: Bot },
  { label: "Total Tasks", value: 9, icon: ListTodo },
  { label: "Escrowed CSPR", value: "66.5 CSPR", icon: Coins },
  { label: "Avg Score", value: "88.0", icon: Star },
];

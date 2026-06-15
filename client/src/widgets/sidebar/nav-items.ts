import {
  LayoutDashboard,
  Bot,
  ListTodo,
  Trophy,
  UserCog,
  PlusCircle,
} from "lucide-react";

export const NAV_ITEMS = [
  {
    section: "Overview",
    items: [
      { href: "/dashboard", label: "Dashboard", icon: LayoutDashboard },
      { href: "/agents", label: "Agents Registry", icon: Bot },
      { href: "/tasks", label: "Job Board", icon: ListTodo },
      { href: "/leaderboard", label: "Leaderboard", icon: Trophy },
    ],
  },
  {
    section: "Operator",
    items: [
      { href: "/my-agent", label: "My Agent", icon: UserCog },
      { href: "/register", label: "Register Bot", icon: PlusCircle },
    ],
  },
] as const;

"use client";

import dynamic from "next/dynamic";
import type { ReactNode } from "react";

const CsprClickProvider = dynamic(
  () => import("@/shared/providers/CsprClickProvider").then((m) => m.CsprClickProvider),
  { ssr: false },
);

export function CsprClickClientWrapper({ children }: { children: ReactNode }) {
  return <CsprClickProvider>{children}</CsprClickProvider>;
}

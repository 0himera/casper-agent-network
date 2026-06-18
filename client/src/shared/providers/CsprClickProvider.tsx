"use client";

import dynamic from "next/dynamic";

export const CsprClickProvider = dynamic(
  () =>
    import("@/features/wallet/ui/CsprClickProvider").then(
      (m) => m.CsprClickProvider,
    ),
  { ssr: false },
);

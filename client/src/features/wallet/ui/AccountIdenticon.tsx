"use client";

import { generateIdenticonSvg } from "../utils/identicon";
import { useMemo } from "react";

export type IdenticonSize = "xs" | "sm" | "m" | "l" | number;

interface AccountIdenticonProps {
  hex: string;
  size?: IdenticonSize;
}

const sizeMap: Record<string, number> = { xs: 16, sm: 20, m: 32, l: 40 };

export function AccountIdenticon({ hex, size = 20 }: AccountIdenticonProps) {
  const px = typeof size === "number" ? size : sizeMap[size] ?? 20;
  const svg = useMemo(() => generateIdenticonSvg(hex, px), [hex, px]);
  return (
    <div
      style={{ width: px, height: px, borderRadius: "50%", overflow: "hidden", flexShrink: 0 }}
      dangerouslySetInnerHTML={{ __html: svg }}
    />
  );
}

"use client";

import "./patch-react-before";
import { type ReactNode } from "react";
import { ThemeProvider } from "styled-components";
import { ClickProvider, DefaultThemes, buildTheme } from "@make-software/csprclick-ui";
import "./patch-react-after";
import { CONTENT_MODE } from "@make-software/csprclick-core-types";

const appTheme = buildTheme({
  csprclickDarkTheme: DefaultThemes.csprclick.csprclickDarkTheme,
  csprclickLightTheme: DefaultThemes.csprclick.csprclickLightTheme,
});

const clickOptions = {
  appName: process.env.NEXT_PUBLIC_APP_NAME || "Casper Agent Network",
  appId: process.env.NEXT_PUBLIC_CLICK_APP_ID || "cspr-agent-network",
  contentMode: CONTENT_MODE.IFRAME,
  providers: ["casper-wallet", "ledger", "metamask-snap"],
};

export function CsprClickProvider({ children }: { children: ReactNode }) {
  return (
    <ThemeProvider theme={appTheme}>
      <ClickProvider options={clickOptions}>{children}</ClickProvider>
    </ThemeProvider>
  );
}

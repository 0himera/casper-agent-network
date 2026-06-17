"use client";

import "./patch-react-before";
import { type ReactNode } from "react";
import { ThemeProvider } from "styled-components";
import { ClickProvider, ClickUI, DefaultThemes, buildTheme, ThemeModeType } from "@make-software/csprclick-ui";
import "./patch-react-after";
import { CONTENT_MODE } from "@make-software/csprclick-core-types";
import { useAppStore } from "@/shared/providers/AppStoreProvider";

const appTheme = buildTheme({
  csprclickDarkTheme: DefaultThemes.csprclick.csprclickDarkTheme,
  csprclickLightTheme: DefaultThemes.csprclick.csprclickLightTheme,
});

const clickOptions = {
  appName: process.env.NEXT_PUBLIC_APP_NAME || "Casper Agent Network",
  appId: process.env.NEXT_PUBLIC_CLICK_APP_ID || "csprclick-template",
  contentMode: CONTENT_MODE.IFRAME,
  providers: ["casper-wallet", "ledger", "metamask-snap"],
};

export function CsprClickProvider({ children }: { children: ReactNode }) {
  const themeMode = useAppStore((s) => s.theme);
  const activeTheme = themeMode === "light" ? appTheme.light : appTheme.dark;

  return (
    <ThemeProvider theme={activeTheme}>
      <ClickProvider options={clickOptions}>
        <ClickUI themeMode={themeMode as ThemeModeType} rootAppElement="body" />
        {children}
      </ClickProvider>
    </ThemeProvider>
  );
}

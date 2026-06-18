"use client";

import { useEffect, useRef, type ReactNode } from "react";
import { CONTENT_MODE } from "@make-software/csprclick-core-types";
import { walletStore } from "../store/wallet-store";

interface CsprClickProviderProps {
  children: ReactNode;
}

const SDK_VERSION = "2.1";

export function CsprClickProvider({ children }: CsprClickProviderProps) {
  const initialized = useRef(false);

  useEffect(() => {
    if (initialized.current) return;
    initialized.current = true;

    const host =
      process.env.NEXT_PUBLIC_CLICK_HOST || "https://cdn.cspr.click/latest";
    const appName =
      process.env.NEXT_PUBLIC_APP_NAME || "Casper Agent Network";
    const appId =
      process.env.NEXT_PUBLIC_CLICK_APP_ID || "csprclick-template";

    window.csprClickSDKAsyncInit = () => {
      window.csprclick.once("csprclick:loaded", () => {
        const acc = window.csprclick.currentAccount;
        walletStore.getState().setInitialized(true);
        if (acc?.public_key) {
          walletStore.getState().setAddress(acc.public_key);
          walletStore.getState().setProvider(acc.provider);
        }
        window.dispatchEvent(new CustomEvent("csprclick:loaded", {}));
      });

      window.csprclick.init({
        appName,
        appId,
        contentMode: CONTENT_MODE.IFRAME,
        providers: ["casper-wallet", "ledger", "metamask-snap"],
      });
    };

    if (!document.getElementById("csprclick-sdk") && !window.csprclick) {
      const script = document.createElement("script");
      script.id = "csprclick-sdk";
      script.src = `${host}/csprclick-sdk-${SDK_VERSION}.js`;
      script.async = true;
      document.head.appendChild(script);
    }
  }, []);

  return <>{children}</>;
}

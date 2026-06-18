import type { ICSPRClickSDK, AccountType } from "@make-software/csprclick-core-types";

declare global {
  interface Window {
    csprclick: ICSPRClickSDK;
    csprClickSDKAsyncInit: () => void;
  }
}

export type {};

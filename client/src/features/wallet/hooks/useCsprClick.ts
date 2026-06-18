"use client";

import { useCallback } from "react";
import { useStore } from "zustand";
import { walletStore, type WalletStore } from "../store/wallet-store";

export function useWalletStore<T>(selector: (s: WalletStore) => T): T {
  return useStore(walletStore, selector);
}

export function useCsprClick() {
  const connect = useCallback(() => {
    window.csprclick?.signIn();
  }, []);

  const switchAccount = useCallback(() => {
    window.csprclick?.switchAccount("");
  }, []);

  const disconnect = useCallback(async () => {
    const acc = window.csprclick?.currentAccount;
    if (acc?.provider) {
      await window.csprclick.disconnect(acc.provider);
    }
    window.csprclick?.signOut();
    walletStore.getState().disconnect();
  }, []);

  return { connect, switchAccount, disconnect };
}

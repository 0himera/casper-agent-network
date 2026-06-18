import { createStore } from "zustand";

export interface WalletState {
  address: string | null;
  isConnected: boolean;
  provider: string | null;
  balance: string | null;
  isInitialized: boolean;
}

export interface WalletActions {
  setAddress: (address: string | null) => void;
  setProvider: (provider: string | null) => void;
  setBalance: (balance: string | null) => void;
  setInitialized: (initialized: boolean) => void;
  disconnect: () => void;
}

export type WalletStore = WalletState & WalletActions;

export const walletStore = createStore<WalletStore>()((set) => ({
  address: null,
  isConnected: false,
  provider: null,
  balance: null,
  isInitialized: false,
  setAddress: (address) => set({ address, isConnected: !!address }),
  setProvider: (provider) => set({ provider }),
  setBalance: (balance) => set({ balance }),
  setInitialized: (isInitialized) => set({ isInitialized }),
  disconnect: () =>
    set({ address: null, isConnected: false, provider: null, balance: null }),
}));

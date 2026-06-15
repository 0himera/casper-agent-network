import { createStore } from "zustand";

export interface AppState {
  theme: "light" | "dark";
  sidebarOpen: boolean;
  commandPaletteOpen: boolean;
  walletAddress: string | null;
}

export interface AppActions {
  setTheme: (theme: "light" | "dark") => void;
  toggleTheme: () => void;
  setSidebarOpen: (open: boolean) => void;
  toggleSidebar: () => void;
  setCommandPaletteOpen: (open: boolean) => void;
  setWalletAddress: (address: string | null) => void;
}

export type AppStore = AppState & AppActions;

export const defaultInitState: AppState = {
  theme: "dark",
  sidebarOpen: true,
  commandPaletteOpen: false,
  walletAddress: null,
};

export const createAppStore = (initState: AppState = defaultInitState) => {
  return createStore<AppStore>()((set) => ({
    ...initState,
    setTheme: (theme) => set({ theme }),
    toggleTheme: () => set((state) => ({ theme: state.theme === "light" ? "dark" : "light" })),
    setSidebarOpen: (sidebarOpen) => set({ sidebarOpen }),
    toggleSidebar: () => set((state) => ({ sidebarOpen: !state.sidebarOpen })),
    setCommandPaletteOpen: (commandPaletteOpen) => set({ commandPaletteOpen }),
    setWalletAddress: (walletAddress) => set({ walletAddress }),
  }));
};

"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { Copy, ExternalLink, LogOut, RefreshCw } from "lucide-react";
import { useAppStore } from "@/shared/providers/AppStoreProvider";
import { truncateAddress, formatCSPR, motesToCSPR } from "@/shared/utils/format";
import { useCsprClick, useWalletStore } from "../hooks/useCsprClick";
import { walletStore } from "../store/wallet-store";
import { AccountIdenticon } from "./AccountIdenticon";
import styles from "@/widgets/navigation/TopBar.module.css";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/shared/ui/dialog";

export function WalletButton() {
  const address = useWalletStore((s) => s.address);
  const isConnected = useWalletStore((s) => s.isConnected);
  const isInitialized = useWalletStore((s) => s.isInitialized);
  const balance = useWalletStore((s) => s.balance);
  const { connect, switchAccount, disconnect } = useCsprClick();
  const setWalletAddress = useAppStore((s) => s.setWalletAddress);
  const [dropdownOpen, setDropdownOpen] = useState(false);
  const [connectModalOpen, setConnectModalOpen] = useState(false);
  const [mockInput, setMockInput] = useState("");
  const dropdownRef = useRef<HTMLDivElement>(null);

  const handleMockConnect = () => {
    if (!mockInput.trim()) return;
    walletStore.getState().setAddress(mockInput.trim());
    walletStore.getState().setProvider("mock");
    walletStore.getState().setBalance("5000000000000"); // 5000 CSPR
    setConnectModalOpen(false);
  };

  useEffect(() => {
    setWalletAddress(address);
  }, [address, setWalletAddress]);

  useEffect(() => {
    if (!isInitialized || !window.csprclick) return;

    const updateAccount = async (accPayload?: any) => {
      const activeAccount = await window.csprclick?.getActiveAccountWithBalance?.();
      const acc = activeAccount || accPayload;
      if (acc) {
        walletStore.getState().setAddress(acc.public_key || acc.publicKey || null);
        walletStore.getState().setProvider(acc.provider || null);
        if (acc.balance !== undefined) {
          walletStore.getState().setBalance(acc.balance);
        }
      } else {
        walletStore.getState().disconnect();
      }
    };

    const onSignedIn = (evt: any) => updateAccount(evt.account || evt);
    const onSwitchedAccount = (evt: any) => updateAccount(evt.account || evt);
    const onSignedOut = () => walletStore.getState().disconnect();
    const onDisconnected = () => walletStore.getState().disconnect();

    const csprclick = window.csprclick;
    csprclick.on("csprclick:signed_in", onSignedIn);
    csprclick.on("csprclick:switched_account", onSwitchedAccount);
    csprclick.on("csprclick:signed_out", onSignedOut);
    csprclick.on("csprclick:disconnected", onDisconnected);

    updateAccount(csprclick.currentAccount);

    return () => {
      csprclick.off("csprclick:signed_in", onSignedIn);
      csprclick.off("csprclick:switched_account", onSwitchedAccount);
      csprclick.off("csprclick:signed_out", onSignedOut);
      csprclick.off("csprclick:disconnected", onDisconnected);
    };
  }, [isInitialized]);

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(e.target as Node)) {
        setDropdownOpen(false);
      }
    };
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);

  const handleCopy = useCallback(async () => {
    if (address) {
      await navigator.clipboard.writeText(address);
      setDropdownOpen(false);
    }
  }, [address]);

  if (!isInitialized) {
    return (
      <button className={`${styles.walletButton} ${styles.disconnected}`} disabled>
        <span>Loading...</span>
      </button>
    );
  }

  if (!isConnected || !address) {
    return (
      <Dialog open={connectModalOpen} onOpenChange={setConnectModalOpen}>
        <DialogTrigger render={<button className={`${styles.walletButton} ${styles.disconnected}`} />}>
          <span>Connect Wallet</span>
        </DialogTrigger>
        <DialogContent className="sm:max-w-md bg-zinc-950 border border-zinc-800 text-zinc-100 shadow-2xl backdrop-blur-xl">
          <DialogHeader>
            <DialogTitle className="text-center text-2xl font-bold mb-4 tracking-tight">Connect Wallet</DialogTitle>
          </DialogHeader>
          <div className="flex flex-col gap-3 py-2">
            <button
              className="flex items-center justify-center p-4 rounded-xl border border-zinc-800 bg-zinc-900/50 hover:bg-zinc-800 hover:border-zinc-700 transition-all cursor-pointer shadow-sm text-zinc-100"
              onClick={() => { setConnectModalOpen(false); connect("casper-wallet"); }}
            >
              <span className="font-semibold text-lg">Casper Wallet</span>
            </button>
            <button
              className="flex items-center justify-center p-4 rounded-xl border border-zinc-800 bg-zinc-900/50 hover:bg-zinc-800 hover:border-zinc-700 transition-all cursor-pointer shadow-sm text-zinc-100"
              onClick={() => { setConnectModalOpen(false); connect("ledger"); }}
            >
              <span className="font-semibold text-lg">Ledger</span>
            </button>
            <button
              className="flex items-center justify-center p-4 rounded-xl border border-zinc-800 bg-zinc-900/50 hover:bg-zinc-800 hover:border-zinc-700 transition-all cursor-pointer shadow-sm text-zinc-100"
              onClick={() => { setConnectModalOpen(false); connect("metamask-snap"); }}
            >
              <span className="font-semibold text-lg">MetaMask Snap</span>
            </button>

            <div className="my-2 border-t border-zinc-800/80 pt-4">
              <div className="text-xs text-zinc-400 mb-2 text-center font-medium uppercase tracking-wider">Development Bypass</div>
              <div className="flex gap-2">
                <input
                  type="text"
                  placeholder="Paste Casper public key..."
                  className="flex-1 px-3 py-2 text-sm bg-zinc-900 border border-zinc-850 rounded-lg text-zinc-100 focus:outline-none focus:border-zinc-700 font-mono"
                  value={mockInput}
                  onChange={(e) => setMockInput(e.target.value)}
                />
                <button
                  onClick={handleMockConnect}
                  className="px-4 py-2 text-sm bg-indigo-600 hover:bg-indigo-500 text-zinc-100 font-semibold rounded-lg transition-all"
                  disabled={!mockInput.trim()}
                >
                  Connect
                </button>
              </div>
              <div className="text-[10px] text-zinc-500 mt-2 text-center">
                Use this if CSPR.click SDK is blocked by an adblocker or fails to load.
              </div>
            </div>
          </div>
        </DialogContent>
      </Dialog>
    );
  }

  return (
    <div className={styles.walletWrapper} ref={dropdownRef}>
      <button
        className={`${styles.walletButton} ${styles.connected}`}
        onClick={() => setDropdownOpen((v) => !v)}
      >
        {balance && (
          <>
            <span className={styles.walletBalance}>
              {balance.includes('CSPR') ? balance : formatCSPR(motesToCSPR(Number(balance)))}
            </span>
            <div className={styles.divider} />
          </>
        )}
        <div className="flex items-center gap-2 bg-black/10 px-2 py-1 rounded-full">
          <AccountIdenticon hex={address} size="xs" />
          <span className={styles.walletAddress}>{truncateAddress(address)}</span>
        </div>
      </button>
      {dropdownOpen && (
        <div className={styles.dropdown}>
          <div className={styles.dropdownIdenticon}>
            <AccountIdenticon hex={address} size="m" />
          </div>
          <div className={styles.dropdownAddr}>{address}</div>
          <button className={styles.dropdownItem} onClick={handleCopy}>
            <Copy size={14} /> Copy Address
          </button>
          <button className={styles.dropdownItem} onClick={switchAccount}>
            <RefreshCw size={14} /> Switch Account
          </button>
          <a
            className={styles.dropdownItem}
            href={`https://cspr.live/account/${address}`}
            target="_blank"
            rel="noopener noreferrer"
          >
            <ExternalLink size={14} /> View on Explorer
          </a>
          <div className={styles.dropdownDivider} />
          <button className={styles.dropdownItem} onClick={disconnect}>
            <LogOut size={14} /> Disconnect
          </button>
        </div>
      )}
    </div>
  );
}

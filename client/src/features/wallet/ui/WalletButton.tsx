"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { Copy, ExternalLink, LogOut, RefreshCw } from "lucide-react";
import { useAppStore } from "@/shared/providers/AppStoreProvider";
import { truncateAddress } from "@/shared/utils/format";
import { useCsprClick, useWalletStore } from "../hooks/useCsprClick";
import { walletStore } from "../store/wallet-store";
import { AccountIdenticon } from "./AccountIdenticon";
import styles from "@/widgets/navigation/TopBar.module.css";

export function WalletButton() {
  const address = useWalletStore((s) => s.address);
  const isConnected = useWalletStore((s) => s.isConnected);
  const isInitialized = useWalletStore((s) => s.isInitialized);
  const { connect, switchAccount, disconnect } = useCsprClick();
  const setWalletAddress = useAppStore((s) => s.setWalletAddress);
  const [dropdownOpen, setDropdownOpen] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    setWalletAddress(address);
  }, [address, setWalletAddress]);

  useEffect(() => {
    if (!isInitialized || !window.csprclick) return;

    const onSignedIn = (evt: any) => {
      const acc = evt.account || evt;
      walletStore.getState().setAddress(acc?.public_key || acc?.publicKey || null);
      walletStore.getState().setProvider(acc?.provider || null);
    };
    const onSwitchedAccount = (evt: any) => {
      const acc = evt.account || evt;
      walletStore.getState().setAddress(acc?.public_key || acc?.publicKey || null);
      walletStore.getState().setProvider(acc?.provider || null);
    };
    const onSignedOut = () => walletStore.getState().disconnect();
    const onDisconnected = () => walletStore.getState().disconnect();

    const csprclick = window.csprclick;
    csprclick.on("csprclick:signed_in", onSignedIn);
    csprclick.on("csprclick:switched_account", onSwitchedAccount);
    csprclick.on("csprclick:signed_out", onSignedOut);
    csprclick.on("csprclick:disconnected", onDisconnected);

    if (csprclick.currentAccount) {
      walletStore.getState().setAddress(csprclick.currentAccount.public_key || null);
      walletStore.getState().setProvider(csprclick.currentAccount.provider || null);
    }

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
      <button
        className={`${styles.walletButton} ${styles.disconnected}`}
        onClick={connect}
      >
        <span>Connect Wallet</span>
      </button>
    );
  }

  return (
    <div className={styles.walletWrapper} ref={dropdownRef}>
      <button
        className={`${styles.walletButton} ${styles.connected}`}
        onClick={() => setDropdownOpen((v) => !v)}
      >
        <AccountIdenticon hex={address} size="xs" />
        <span className={styles.walletAddress}>{truncateAddress(address)}</span>
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

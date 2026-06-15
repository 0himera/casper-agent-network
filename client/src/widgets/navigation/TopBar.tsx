"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { Wallet, Bell, Copy, ExternalLink, LogOut } from "lucide-react";
import { useAppStore } from "@/shared/providers/AppStoreProvider";
import { truncateAddress } from "@/shared/utils/format";
import styles from "./TopBar.module.css";

import { connectWallet, disconnectWallet, getConnectedAccount } from "@/shared/utils/casper-wallet";

interface TopBarProps {
  title?: string;
}

export function TopBar({ title }: TopBarProps) {
  const sidebarOpen = useAppStore((s) => s.sidebarOpen);
  const walletAddress = useAppStore((s) => s.walletAddress);
  const setWalletAddress = useAppStore((s) => s.setWalletAddress);
  const isConnected = !!walletAddress;

  const [dropdownOpen, setDropdownOpen] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    getConnectedAccount().then((address) => {
      if (address) setWalletAddress(address);
    });
  }, [setWalletAddress]);

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(e.target as Node)) {
        setDropdownOpen(false);
      }
    };
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);

  const handleConnect = async () => {
    const address = await connectWallet();
    if (address) setWalletAddress(address);
  };

  const handleDisconnect = useCallback(async () => {
    await disconnectWallet();
    setWalletAddress(null);
    setDropdownOpen(false);
  }, [setWalletAddress]);

  const handleCopy = useCallback(async () => {
    if (walletAddress) {
      await navigator.clipboard.writeText(walletAddress);
      setDropdownOpen(false);
    }
  }, [walletAddress]);

  return (
    <header
      className={`${styles.topbar} ${sidebarOpen ? styles.expanded : styles.collapsed}`}
    >
      <div className={styles.leftSection}>
        {title && <h1 className={styles.pageTitle}>{title}</h1>}
      </div>

      <div className={styles.rightSection}>
        <button className={styles.iconButton} aria-label="Notifications">
          <Bell size={18} />
        </button>

        {isConnected ? (
          <div className={styles.walletWrapper} ref={dropdownRef}>
            <button
              className={`${styles.walletButton} ${styles.connected}`}
              onClick={() => setDropdownOpen((v) => !v)}
            >
              <div className={styles.walletDot} />
              <span className={styles.walletAddress}>
                {truncateAddress(walletAddress)}
              </span>
            </button>
            {dropdownOpen && (
              <div className={styles.dropdown}>
                <div className={styles.dropdownAddr}>{walletAddress}</div>
                <button className={styles.dropdownItem} onClick={handleCopy}>
                  <Copy size={14} /> Copy Address
                </button>
                <a
                  className={styles.dropdownItem}
                  href={`https://cspr.live/account/${walletAddress}`}
                  target="_blank"
                  rel="noopener noreferrer"
                >
                  <ExternalLink size={14} /> View on Explorer
                </a>
                <div className={styles.dropdownDivider} />
                <button className={styles.dropdownItem} onClick={handleDisconnect}>
                  <LogOut size={14} /> Disconnect
                </button>
              </div>
            )}
          </div>
        ) : (
          <button
            className={`${styles.walletButton} ${styles.disconnected}`}
            onClick={handleConnect}
          >
            <Wallet size={16} />
            <span>Connect Wallet</span>
          </button>
        )}
      </div>
    </header>
  );
}

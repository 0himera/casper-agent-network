"use client";

import { Wallet, Bell } from "lucide-react";
import { useAppStore } from "@/shared/providers/AppStoreProvider";
import { truncateAddress } from "@/shared/utils/format";
import styles from "./TopBar.module.css";

interface TopBarProps {
  title?: string;
}

export function TopBar({ title }: TopBarProps) {
  const sidebarOpen = useAppStore((s) => s.sidebarOpen);
  const walletAddress = useAppStore((s) => s.walletAddress);
  const setWalletAddress = useAppStore((s) => s.setWalletAddress);
  const isConnected = !!walletAddress;

  const handleWalletClick = () => {
    if (isConnected) {
      setWalletAddress(null);
    } else {
      setWalletAddress("01a4b3c2d1e0f9876543210abcdef0123456789abcdef0123456789abcdef0123");
    }
  };

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

        <button
          className={`${styles.walletButton} ${isConnected ? styles.connected : styles.disconnected}`}
          onClick={handleWalletClick}
        >
          {isConnected ? (
            <>
              <div className={styles.walletDot} />
              <span className={styles.walletAddress}>
                {truncateAddress(walletAddress)}
              </span>
            </>
          ) : (
            <>
              <Wallet size={16} />
              <span>Connect Wallet</span>
            </>
          )}
        </button>
      </div>
    </header>
  );
}

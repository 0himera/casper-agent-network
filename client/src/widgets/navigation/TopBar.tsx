"use client";

import "@/shared/providers/patch-react-before";
import { useClickRef, AccountIdenticon } from "@make-software/csprclick-ui";
import "@/shared/providers/patch-react-after";
import { useCallback, useEffect, useRef, useState } from "react";
import { Bell, Copy, ExternalLink, LogOut } from "lucide-react";
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

  const [dropdownOpen, setDropdownOpen] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);
  const clickRef = useClickRef();

  useEffect(() => {
    if (!clickRef) return;

    const handleSignedIn = (evt: any) => {
      const acc = evt.account || evt;
      setWalletAddress(acc?.public_key || acc?.publicKey || null);
    };
    const handleSwitchedAccount = (evt: any) => {
      const acc = evt.account || evt;
      setWalletAddress(acc?.public_key || acc?.publicKey || null);
    };
    const handleSignedOut = () => setWalletAddress(null);

    clickRef.on("csprclick:signed_in", handleSignedIn);
    clickRef.on("csprclick:switched_account", handleSwitchedAccount);
    clickRef.on("csprclick:signed_out", handleSignedOut);

    return () => {
      clickRef.off("csprclick:signed_in", handleSignedIn);
      clickRef.off("csprclick:switched_account", handleSwitchedAccount);
      clickRef.off("csprclick:signed_out", handleSignedOut);
    };
  }, [clickRef, setWalletAddress]);

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(e.target as Node)) {
        setDropdownOpen(false);
      }
    };
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);

  const handleConnect = () => {
    window.csprclick?.signIn();
  };

  const handleDisconnect = useCallback(async () => {
    if (window.csprclick?.currentAccount?.provider) {
      await window.csprclick.disconnect(window.csprclick.currentAccount.provider);
    }
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
              <AccountIdenticon hex={walletAddress} size="xs" />
              <span className={styles.walletAddress}>
                {truncateAddress(walletAddress)}
              </span>
            </button>
            {dropdownOpen && (
              <div className={styles.dropdown}>
                <div className={styles.dropdownIdenticon}>
                  <AccountIdenticon hex={walletAddress} size="m" />
                </div>
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
            <span>Connect Wallet</span>
          </button>
        )}
      </div>
    </header>
  );
}

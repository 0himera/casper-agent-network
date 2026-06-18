"use client";

import { Bell } from "lucide-react";
import { useAppStore } from "@/shared/providers/AppStoreProvider";
import { WalletButton } from "@/features/wallet/ui/WalletButton";
import styles from "./TopBar.module.css";

interface TopBarProps {
  title?: string;
}

export function TopBar({ title }: TopBarProps) {
  const sidebarOpen = useAppStore((s) => s.sidebarOpen);

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

        <WalletButton />
      </div>
    </header>
  );
}

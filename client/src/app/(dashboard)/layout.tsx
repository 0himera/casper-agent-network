"use client";

import { useAppStore } from "@/shared/providers/AppStoreProvider";
import { Sidebar } from "@/widgets/sidebar/Sidebar";
import { TopBar } from "@/widgets/navigation/TopBar";
import styles from "./DashboardLayout.module.css";

export default function DashboardLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const sidebarOpen = useAppStore((s) => s.sidebarOpen);

  return (
    <div className={styles.layout}>
      <Sidebar />
      <div
        className={`${styles.mainArea} ${sidebarOpen ? styles.expanded : styles.collapsed}`}
      >
        <TopBar />
        <main className={styles.content}>
          {children}
        </main>
      </div>
    </div>
  );
}

"use client";

import { usePathname } from "next/navigation";
import { useAppStore } from "@/shared/providers/AppStoreProvider";
import { Sidebar } from "@/widgets/sidebar/Sidebar";
import { TopBar } from "@/widgets/navigation/TopBar";
import { motion, AnimatePresence } from "motion/react";
import styles from "./DashboardLayout.module.css";

export default function DashboardLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const sidebarOpen = useAppStore((s) => s.sidebarOpen);
  const pathname = usePathname();

  return (
    <div className={styles.layout}>
      <Sidebar />
      <div
        className={`${styles.mainArea} ${sidebarOpen ? styles.expanded : styles.collapsed}`}
      >
        <TopBar />
        <main className={styles.content}>
          <AnimatePresence mode="wait">
            <motion.div
              key={pathname}
              initial={{ opacity: 0, y: 15 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -15 }}
              transition={{ duration: 0.25, ease: [0.16, 1, 0.3, 1] }}
              style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column" }}
            >
              {children}
            </motion.div>
          </AnimatePresence>
        </main>
      </div>
    </div>
  );
}

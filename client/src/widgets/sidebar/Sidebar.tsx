"use client";

import { usePathname } from "next/navigation";
import { PanelLeftClose, PanelLeft , Shield, Landmark } from "lucide-react";
import { useAppStore } from "@/shared/providers/AppStoreProvider";
import { NAV_ITEMS } from "./nav-items";
import { SidebarNavLink } from "./SidebarNavLink";
import styles from "./Sidebar.module.css";

export function Sidebar() {
  const pathname = usePathname();
  const sidebarOpen = useAppStore((s) => s.sidebarOpen);
  const toggleSidebar = useAppStore((s) => s.toggleSidebar);
  const cls = `${styles.sidebar} ${!sidebarOpen ? styles.collapsed : ""}`;

  return (
    <aside className={cls}>
      <div className={styles.logoArea}>
        <div 
          className={styles.logoIcon}
          style={{ 
            background: 'var(--text-primary)',
            WebkitMaskImage: 'url(/logo.svg)', 
            WebkitMaskSize: 'contain', 
            WebkitMaskPosition: 'center',
            WebkitMaskRepeat: 'no-repeat', 
            maskImage: 'url(/logo.svg)', 
            maskSize: 'contain', 
            maskPosition: 'center',
            maskRepeat: 'no-repeat' 
          }} 
        />
        <div className={styles.logoText}>
          <span className={styles.logoTitle}>Casper Agent</span>
          <span className={styles.logoSubtitle}>Network</span>
        </div>
      </div>

      <nav className={styles.nav}>

        {NAV_ITEMS.map((section) => (
          <div key={section.section} className={styles.navSection}>
            <div className={styles.navSectionLabel}>{section.section}</div>
            {section.items.map((item) => (
              <SidebarNavLink
                key={item.href}
                href={item.href}
                label={item.label}
                icon={item.icon}
                isActive={pathname === item.href || pathname.startsWith(item.href + "/")}
                isCollapsed={!sidebarOpen}
              />
            ))}
          </div>
        ))}
      </nav>

      <div className={styles.bottomSection}>
        <div className={styles.networkBadge}>
          <div className={styles.networkDot} />
          <span>Casper Testnet</span>
        </div>
      </div>

      <button
        onClick={toggleSidebar}
        className={styles.navLink}
        style={{ margin: "0 8px 12px", flexShrink: 0 }}
        aria-label={sidebarOpen ? "Collapse sidebar" : "Expand sidebar"}
      >
        {sidebarOpen ? <PanelLeftClose className={styles.navIcon} /> : <PanelLeft className={styles.navIcon} />}
        <span className={styles.navLabel}>{sidebarOpen ? "Collapse" : ""}</span>
      </button>
    </aside>
  );
}

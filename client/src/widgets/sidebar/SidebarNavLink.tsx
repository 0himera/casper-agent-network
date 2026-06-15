import Link from "next/link";
import type { LucideIcon } from "lucide-react";
import { motion } from "motion/react";
import styles from "./Sidebar.module.css";

interface NavLinkProps {
  href: string;
  label: string;
  icon: LucideIcon;
  isActive: boolean;
  isCollapsed: boolean;
}

export function SidebarNavLink({ href, label, icon: Icon, isActive, isCollapsed }: NavLinkProps) {
  return (
    <Link
      href={href}
      className={`${styles.navLink} ${isActive ? styles.active : ""}`}
    >
      {isActive && (
        <motion.div
          layoutId="activeSidebarTab"
          className={styles.activeBackground}
          transition={{ type: "spring", stiffness: 380, damping: 30 }}
        />
      )}
      <Icon className={styles.navIcon} style={{ position: "relative", zIndex: 2 }} />
      <span className={styles.navLabel} style={{ position: "relative", zIndex: 2 }}>{label}</span>
    </Link>
  );
}

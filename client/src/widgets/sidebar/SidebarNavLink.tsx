import Link from "next/link";
import type { LucideIcon } from "lucide-react";
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
      <Icon className={styles.navIcon} />
      <span className={styles.navLabel}>{label}</span>
    </Link>
  );
}

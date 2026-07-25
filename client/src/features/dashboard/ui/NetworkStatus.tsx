"use client";

import type { LucideIcon } from "lucide-react";
import { Cpu, ShieldCheck, Globe } from "lucide-react";
import { useHealthQuery } from "@/features/dashboard/api/queries";
import { Skeleton } from "@/shared/ui";
import styles from "./Dashboard.module.css";

interface NetworkStatusProps {
  dataHealthy?: boolean;
}

interface ServiceItem {
  label: string;
  icon: LucideIcon;
  healthy: boolean;
  description: string;
}

export function NetworkStatus({ dataHealthy }: NetworkStatusProps) {
  const { data: health, isLoading } = useHealthQuery();
  const backendHealthy = health?.status === "ok";
  const isDataHealthy = !!dataHealthy;

  const services: ServiceItem[] = [
    {
      label: "Casper Testnet",
      icon: Globe,
      healthy: true,
      description: "Live testnet contract is reachable",
    },
    {
      label: "Registry API",
      icon: Cpu,
      healthy: backendHealthy && isDataHealthy,
      description: "Backend registry and task endpoints",
    },
    {
      label: "Judge Engine",
      icon: ShieldCheck,
      healthy: backendHealthy,
      description: "LLM-as-a-Judge validation pipeline",
    },
  ];

  return (
    <div className={styles.dashboardSection}>
      <h3 className={styles.sectionTitle}>Network Status</h3>
      <div className={styles.systemStatusGrid}>
        {isLoading
          ? Array.from({ length: 3 }).map((_, i) => (
              <div key={i} className={styles.systemStatusCard}>
                <Skeleton width={16} height={16} borderRadius="50%" />
                <Skeleton width="60%" height={12} />
              </div>
            ))
          : services.map((s) => (
              <div
                key={s.label}
                className={`${styles.systemStatusCard} ${
                  s.healthy ? styles.systemStatusOnline : styles.systemStatusOffline
                }`}
                title={s.description}
              >
                <span
                  className={`${styles.statusDot} ${
                    s.healthy ? styles.statusDotOnline : styles.statusDotOffline
                  }`}
                  aria-hidden="true"
                />
                <s.icon size={14} className={styles.statusIcon} aria-hidden="true" />
                <span>{s.label}</span>
              </div>
            ))}
      </div>
    </div>
  );
}

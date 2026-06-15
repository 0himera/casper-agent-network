"use client";

import styles from "./Skeleton.module.css";

interface SkeletonProps extends React.HTMLAttributes<HTMLDivElement> {
  width?: string | number;
  height?: string | number;
  borderRadius?: string | number;
}

export function Skeleton({
  className = "",
  width,
  height,
  borderRadius,
  style,
  ...props
}: SkeletonProps) {
  return (
    <div
      className={`${styles.skeleton} ${className}`}
      style={{
        width,
        height,
        borderRadius: borderRadius ?? "var(--radius-sm)",
        ...style,
      }}
      {...props}
    />
  );
}

export function SkeletonCardGrid({ count = 6 }: { count?: number }) {
  return (
    <div className={styles.grid}>
      {Array.from({ length: count }).map((_, i) => (
        <div key={i} className={styles.card}>
          <div className={styles.cardHeader}>
            <Skeleton width={40} height={40} borderRadius="var(--radius-sm)" />
            <div className={styles.headerInfo}>
              <Skeleton width="60%" height={14} />
              <Skeleton width="35%" height={10} />
            </div>
            <Skeleton width={60} height={18} borderRadius="var(--radius-full)" />
          </div>
          <div className={styles.cardBody}>
            <Skeleton width="95%" height={12} />
            <Skeleton width="75%" height={12} />
          </div>
          <div style={{ display: "flex", gap: "6px", flexWrap: "wrap" }}>
            <Skeleton width={55} height={16} borderRadius="var(--radius-full)" />
            <Skeleton width={70} height={16} borderRadius="var(--radius-full)" />
          </div>
          <div className={styles.cardFooter}>
            <div className={styles.footerItem}>
              <Skeleton width={30} height={8} />
              <Skeleton width={45} height={12} />
            </div>
            <div className={styles.footerItem}>
              <Skeleton width={30} height={8} />
              <Skeleton width={45} height={12} />
            </div>
            <div className={styles.footerItem}>
              <Skeleton width={30} height={8} />
              <Skeleton width={45} height={12} />
            </div>
          </div>
        </div>
      ))}
    </div>
  );
}

export function SkeletonTable({ rows = 5 }: { rows?: number }) {
  return (
    <div className={styles.tableContainer}>
      <div className={styles.tableHeader}>
        <Skeleton width="15%" height={14} />
        <Skeleton width="8%" height={14} />
      </div>
      <div style={{ display: "flex", flexDirection: "column" }}>
        {Array.from({ length: rows }).map((_, i) => (
          <div key={i} className={styles.tableRow}>
            <div className={styles.rowLeft}>
              <Skeleton width={20} height={14} />
              <Skeleton width={32} height={32} borderRadius="var(--radius-sm)" />
              <div className={styles.rowLeftInfo}>
                <Skeleton width="25%" height={12} />
                <Skeleton width="15%" height={8} />
              </div>
            </div>
            <div className={styles.rowRight}>
              <Skeleton width={60} height={12} />
              <Skeleton width={80} height={12} />
              <Skeleton width={50} height={12} />
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

export function SkeletonDetail() {
  return (
    <div className={styles.detailContainer}>
      <Skeleton width={120} height={14} />
      <div className={styles.detailHero}>
        <Skeleton width={64} height={64} borderRadius="var(--radius-md)" style={{ flexShrink: 0 }} />
        <div className={styles.detailInfo}>
          <Skeleton width="30%" height={20} />
          <Skeleton width="20%" height={10} style={{ margin: "4px 0" }} />
          <Skeleton width="90%" height={14} style={{ marginTop: "8px" }} />
          <Skeleton width="70%" height={14} />
        </div>
      </div>
      <div className={styles.detailStatsGrid}>
        {Array.from({ length: 4 }).map((_, i) => (
          <div key={i} className={styles.detailStatBox}>
            <Skeleton width="50%" height={10} />
            <Skeleton width="70%" height={18} />
          </div>
        ))}
      </div>
    </div>
  );
}

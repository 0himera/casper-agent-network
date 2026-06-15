"use client";

import { useAgentBenchmarksQuery } from "@/features/agents/api/queries";
import styles from "./MyAgent.module.css";

interface BenchmarkPanelProps {
  publicKey: string;
}

export function BenchmarkPanel({ publicKey }: BenchmarkPanelProps) {
  const { data: runs, isLoading } = useAgentBenchmarksQuery(publicKey);

  if (isLoading) {
    return (
      <div className={styles.section}>
        <h3 className={styles.sectionTitle}>Benchmark Performance</h3>
        <div style={{ color: "var(--text-muted)", opacity: 0.6 }}>Loading performance metrics...</div>
      </div>
    );
  }

  const latestRun = runs && runs.length > 0 ? runs[0] : null;

  if (!latestRun) {
    return (
      <div className={styles.section}>
        <h3 className={styles.sectionTitle}>Benchmark Performance</h3>
        <div style={{ color: "var(--text-muted)", opacity: 0.6, fontSize: "0.9rem" }}>
          No benchmark runs recorded yet. The validator evaluates your agent upon registration and updates.
        </div>
      </div>
    );
  }

  const criteria = Array.isArray(latestRun.rubric_scores)
    ? latestRun.rubric_scores
    : typeof latestRun.rubric_scores === "object" && latestRun.rubric_scores !== null
    ? Object.values(latestRun.rubric_scores)
    : [];

  return (
    <div className={styles.section}>
      <h3 className={styles.sectionTitle}>Benchmark Performance</h3>
      <div className={styles.benchGrid}>
        <span className={styles.benchLabel}>Last Run:</span>
        <span className={styles.benchValue}>
          {new Date(latestRun.timestamp).toLocaleString()}
        </span>
        
        {criteria.map((c: any) => (
          <div key={c.id || Math.random()} style={{ display: "contents" }}>
            <span className={styles.benchLabel}>
              {(c.id || "Criterion").replace(/_/g, " ").replace(/\b\w/g, (l: string) => l.toUpperCase())}:
            </span>
            <span className={styles.benchValue}>
              <span
                style={{
                  color: c.passed ? "#10b981" : "#ef4444",
                  fontWeight: 600,
                  marginRight: "8px",
                }}
              >
                {c.passed ? "✓ Passed" : "✗ Failed"}
              </span>
              ({c.score} pts)
            </span>
          </div>
        ))}
      </div>
      <div className={styles.benchScore}>
        Total Score: {latestRun.score}/100
      </div>
    </div>
  );
}

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

  let criteria: any[] = [];
  if (latestRun.rubric_scores) {
    if (Array.isArray(latestRun.rubric_scores)) {
      criteria = latestRun.rubric_scores;
    } else if (Array.isArray(latestRun.rubric_scores.criteria)) {
      criteria = latestRun.rubric_scores.criteria;
    } else if (typeof latestRun.rubric_scores === "object") {
      criteria = Object.entries(latestRun.rubric_scores)
        .filter(([key]) => !["pipeline", "verdict", "total", "explanation", "stages", "stats"].includes(key))
        .map(([key, val]: [string, any]) => {
          let score = 0;
          let passed = false;
          if (typeof val === "number") {
            score = val;
            passed = val > 0;
          } else if (val && typeof val === "object") {
            score = typeof val.score === "number" ? val.score : 0;
            passed = typeof val.passed === "boolean" ? val.passed : score > 0;
          }
          return { id: key, score, passed };
        });
    }
  }

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

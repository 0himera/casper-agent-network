"use client";

import { useAgentBenchmarksQuery } from "@/features/agents/api/queries";
import type { BenchmarkCriterion, BenchmarkRun } from "@/entities/agent/types/types";
import styles from "./MyAgent.module.css";

interface BenchmarkPanelProps {
  publicKey: string;
}

const IGNORED_RUBRIC_KEYS = ["pipeline", "verdict", "total", "explanation", "stages", "stats"];

function isBenchmarkCriterion(value: unknown): value is BenchmarkCriterion {
  return (
    typeof value === "object" &&
    value !== null &&
    typeof (value as BenchmarkCriterion).id === "string" &&
    typeof (value as BenchmarkCriterion).score === "number" &&
    typeof (value as BenchmarkCriterion).passed === "boolean"
  );
}

function normalizeRubricCriteria(rubricScores: unknown): BenchmarkCriterion[] {
  if (Array.isArray(rubricScores)) {
    return rubricScores.filter(isBenchmarkCriterion);
  }

  if (typeof rubricScores === "object" && rubricScores !== null) {
    const record = rubricScores as Record<string, unknown>;

    if (Array.isArray(record.criteria)) {
      return record.criteria.filter(isBenchmarkCriterion);
    }

    return Object.entries(record)
      .filter(([key]) => !IGNORED_RUBRIC_KEYS.includes(key))
      .map(([key, val]) => {
        let score = 0;
        let passed = false;

        if (typeof val === "number") {
          score = val;
          passed = val > 0;
        } else if (val && typeof val === "object") {
          const v = val as Record<string, unknown>;
          score = typeof v.score === "number" ? v.score : 0;
          passed = typeof v.passed === "boolean" ? v.passed : score > 0;
        }

        return { id: key, score, passed };
      });
  }

  return [];
}

export function BenchmarkPanel({ publicKey }: BenchmarkPanelProps) {
  const { data: runs, isLoading } = useAgentBenchmarksQuery(publicKey);

  if (isLoading) {
    return (
      <div className={styles.section}>
        <h3 className={styles.sectionTitle}>Benchmark Performance</h3>
        <div style={{ color: "var(--text-muted)", opacity: 0.6 }}>
          Loading performance metrics...
        </div>
      </div>
    );
  }

  const latestRun: BenchmarkRun | null = runs && runs.length > 0 ? runs[0] : null;

  if (!latestRun) {
    return (
      <div className={styles.section}>
        <h3 className={styles.sectionTitle}>Benchmark Performance</h3>
        <div style={{ color: "var(--text-muted)", opacity: 0.6, fontSize: "0.9rem" }}>
          No benchmark runs recorded yet. The validator evaluates your agent upon registration and
          updates.
        </div>
      </div>
    );
  }

  const criteria = normalizeRubricCriteria(latestRun.rubric_scores);

  return (
    <div className={styles.section}>
      <h3 className={styles.sectionTitle}>Benchmark Performance</h3>
      <div className={styles.benchGrid}>
        <span className={styles.benchLabel}>Last Run:</span>
        <span className={styles.benchValue}>{new Date(latestRun.timestamp).toLocaleString()}</span>

        {criteria.map((c, i) => (
          <div key={c.id || i} style={{ display: "contents" }}>
            <span className={styles.benchLabel}>
              {(c.id || "Criterion").replace(/_/g, " ").replace(/\b\w/g, (l) => l.toUpperCase())}:
            </span>
            <span className={styles.benchValue}>
              <span
                style={{
                  color: c.passed ? "#10b981" : "#ef4444",
                  fontWeight: 600,
                  marginRight: "8px",
                }}
              >
                {c.passed ? "Passed" : "Failed"}
              </span>
              ({c.score} pts)
            </span>
          </div>
        ))}
      </div>
      <div className={styles.benchScore}>Total Score: {latestRun.score ?? 0}/100</div>
    </div>
  );
}

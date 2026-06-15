import styles from "./MyAgent.module.css";

export function BenchmarkPanel() {
  return (
    <div className={styles.section}>
      <h3 className={styles.sectionTitle}>Benchmark Performance</h3>
      <div className={styles.benchGrid}>
        <span className={styles.benchLabel}>Last Run:</span>
        <span className={styles.benchValue}>2026-06-15 14:30</span>
        <span className={styles.benchLabel}>Accuracy:</span>
        <span className={styles.benchValue}>28/30</span>
        <span className={styles.benchLabel}>Depth:</span>
        <span className={styles.benchValue}>22/25</span>
        <span className={styles.benchLabel}>Sources:</span>
        <span className={styles.benchValue}>18/20</span>
        <span className={styles.benchLabel}>Actionability:</span>
        <span className={styles.benchValue}>14/15</span>
        <span className={styles.benchLabel}>Presentation:</span>
        <span className={styles.benchValue}>10/10</span>
      </div>
      <div className={styles.benchScore}>Total Score: 92/100</div>
    </div>
  );
}

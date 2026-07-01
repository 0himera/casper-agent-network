import React from "react";
import styles from "@/features/leaderboard/ui/Leaderboard.module.css";

export default function ValidatorsPage() {
  const mockValidators = [
    { pk: "01c9a...3f12", stake: "50,000 CSPR", status: "Active", accuracy: "99.4%" },
    { pk: "021b4...8a9c", stake: "24,500 CSPR", status: "Active", accuracy: "98.1%" },
    { pk: "01f3e...b411", stake: "12,000 CSPR", status: "Active", accuracy: "97.5%" },
    { pk: "01a44...c990", stake: "5,000 CSPR", status: "Slashed", accuracy: "62.0%" }
  ];

  return (
    <div className={styles.page}>
      <div className={styles.header}>
        <h1 className={styles.title}>Yuma-Lite Validator Network</h1>
        <p className={styles.subtitle}>
          Decentralized LLM nodes running Median Consensus to grade AI Swarm Agents.
        </p>
      </div>
      
      <div style={{ marginTop: "2rem", width: "100%", maxWidth: "800px" }}>
        <table style={{ width: "100%", borderCollapse: "collapse", textAlign: "left" }}>
          <thead>
            <tr style={{ borderBottom: "1px solid #333", color: "#888" }}>
              <th style={{ padding: "12px 0" }}>Validator PubKey</th>
              <th style={{ padding: "12px 0" }}>Total Stake</th>
              <th style={{ padding: "12px 0" }}>Status</th>
              <th style={{ padding: "12px 0" }}>Consensus Accuracy</th>
            </tr>
          </thead>
          <tbody>
            {mockValidators.map((v, i) => (
              <tr key={i} style={{ borderBottom: "1px solid #222" }}>
                <td style={{ padding: "16px 0", fontFamily: "monospace", color: "#fff" }}>{v.pk}</td>
                <td style={{ padding: "16px 0", color: "#a5d6ff" }}>{v.stake}</td>
                <td style={{ padding: "16px 0" }}>
                  <span style={{ 
                    padding: "4px 8px", 
                    borderRadius: "12px", 
                    fontSize: "0.8rem",
                    backgroundColor: v.status === "Active" ? "rgba(46, 160, 67, 0.2)" : "rgba(248, 81, 73, 0.2)",
                    color: v.status === "Active" ? "#3fb950" : "#f85149"
                  }}>
                    {v.status}
                  </span>
                </td>
                <td style={{ padding: "16px 0", color: "#fff" }}>{v.accuracy}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

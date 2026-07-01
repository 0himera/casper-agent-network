import React from "react";
import styles from "@/features/leaderboard/ui/Leaderboard.module.css";

export default function TreasuryPage() {
  const recentEvents = [
    { type: "Burn", amount: "5,000 CSPR", date: "2026-07-01 14:00" },
    { type: "Yield Distribution", amount: "2,500 CSPR", date: "2026-07-01 12:00" },
    { type: "Fee Collection", amount: "150 CSPR", date: "2026-07-01 11:45" },
    { type: "Slashing (Missed Deadline)", amount: "50 CSPR", date: "2026-07-01 10:20" },
  ];

  return (
    <div className={styles.page}>
      <div className={styles.header}>
        <h1 className={styles.title}>Protocol Treasury</h1>
        <p className={styles.subtitle}>
          Deflationary tokenomics driven by Agent Fees and Validator Slashing.
        </p>
      </div>

      <div style={{ display: "flex", gap: "2rem", marginTop: "2rem" }}>
        <div style={{ padding: "24px", background: "#161b22", borderRadius: "12px", border: "1px solid #30363d", flex: 1 }}>
          <h3 style={{ color: "#888", marginBottom: "8px", fontSize: "14px" }}>Total Escrowed Treasury</h3>
          <div style={{ fontSize: "2rem", fontWeight: "bold", color: "#a5d6ff" }}>142,500 CSPR</div>
        </div>
        <div style={{ padding: "24px", background: "#161b22", borderRadius: "12px", border: "1px solid #30363d", flex: 1 }}>
          <h3 style={{ color: "#888", marginBottom: "8px", fontSize: "14px" }}>Tokens Burned (Deflation)</h3>
          <div style={{ fontSize: "2rem", fontWeight: "bold", color: "#f85149" }}>12,000 CSPR</div>
        </div>
      </div>
      
      <div style={{ marginTop: "3rem", width: "100%", maxWidth: "800px" }}>
        <h2 style={{ fontSize: "1.2rem", marginBottom: "1rem" }}>Recent Treasury Events</h2>
        <table style={{ width: "100%", borderCollapse: "collapse", textAlign: "left" }}>
          <thead>
            <tr style={{ borderBottom: "1px solid #333", color: "#888" }}>
              <th style={{ padding: "12px 0" }}>Event Type</th>
              <th style={{ padding: "12px 0" }}>Amount</th>
              <th style={{ padding: "12px 0" }}>Date</th>
            </tr>
          </thead>
          <tbody>
            {recentEvents.map((evt, i) => (
              <tr key={i} style={{ borderBottom: "1px solid #222" }}>
                <td style={{ padding: "16px 0", color: "#fff" }}>
                  <span style={{ 
                    padding: "4px 8px", 
                    borderRadius: "12px", 
                    fontSize: "0.8rem",
                    backgroundColor: evt.type === "Burn" ? "rgba(248, 81, 73, 0.2)" : 
                                     evt.type.includes("Yield") ? "rgba(46, 160, 67, 0.2)" : "rgba(56, 139, 253, 0.1)",
                    color: evt.type === "Burn" ? "#f85149" : 
                           evt.type.includes("Yield") ? "#3fb950" : "#58a6ff"
                  }}>
                    {evt.type}
                  </span>
                </td>
                <td style={{ padding: "16px 0", color: "#a5d6ff", fontFamily: "monospace" }}>{evt.amount}</td>
                <td style={{ padding: "16px 0", color: "#888" }}>{evt.date}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

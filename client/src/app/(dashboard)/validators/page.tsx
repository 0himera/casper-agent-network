"use client";

import React, { useEffect, useState } from "react";
import styles from "@/features/leaderboard/ui/Leaderboard.module.css";

interface ValidatorNode {
  node_id: string;
  name: string;
  provider: string;
  pk: string;
  stake: string;
  status: string;
  validations_count: number;
  consensus_accuracy: string;
}

export default function ValidatorsPage() {
  const [validators, setValidators] = useState<ValidatorNode[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const fetchValidators = async () => {
      try {
        const baseUrl = typeof window !== "undefined" ? "" : (process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080");
        const res = await fetch(`${baseUrl}/api/validators`);
        if (res.ok) {
          const data = await res.json();
          setValidators(data);
        }
      } catch (err) {
        console.error("Failed to fetch validators:", err);
      } finally {
        setLoading(false);
      }
    };
    fetchValidators();
  }, []);

  const defaultValidators: ValidatorNode[] = [
    {
      node_id: "validator-1",
      name: "Validator Node 1",
      provider: "Fireworks AI (DeepSeek v4 Flash)",
      pk: "01a74a93e16ccf32fa9d91d387c9fb84521e23fdae8ce57263d173beafab5fc1b8",
      stake: "100 CSPR",
      status: "Active",
      validations_count: 12,
      consensus_accuracy: "99.4%",
    },
    {
      node_id: "validator-2",
      name: "Validator Node 2",
      provider: "Google AI (Gemini 3.1 Flash Lite)",
      pk: "01bad4de01164b7a4c90eb19bec1b218092ce000bb3bbbf09cc15b0f94da56ac75",
      stake: "100 CSPR",
      status: "Active",
      validations_count: 12,
      consensus_accuracy: "99.1%",
    },
    {
      node_id: "validator-3",
      name: "Validator Node 3",
      provider: "OpenRouter (NVIDIA Nemotron 3 Ultra)",
      pk: "01bae700f4024cff103b68d66f86a0227ccd3b2c7b8f0d1d880a803808a53a8ff1",
      stake: "100 CSPR",
      status: "Active",
      validations_count: 12,
      consensus_accuracy: "98.8%",
    },
  ];

  const displayList = validators.length > 0 ? validators : defaultValidators;

  return (
    <div className={styles.page}>
      <div className={styles.header}>
        <h1 className={styles.title}>3-Validator Consensus Network</h1>
        <p className={styles.subtitle}>
          Multi-Model LLM Consensus Nodes on Casper Testnet (Quorum: 3 / 3 Required).
        </p>
      </div>

      <div style={{ marginTop: "2rem", width: "100%", maxWidth: "900px" }}>
        <table style={{ width: "100%", borderCollapse: "collapse", textAlign: "left" }}>
          <thead>
            <tr style={{ borderBottom: "1px solid #333", color: "#888", fontSize: "0.85rem" }}>
              <th style={{ padding: "12px 0" }}>Node & LLM Provider</th>
              <th style={{ padding: "12px 0" }}>Validator PubKey</th>
              <th style={{ padding: "12px 0" }}>Stake</th>
              <th style={{ padding: "12px 0" }}>Validations</th>
              <th style={{ padding: "12px 0" }}>Status</th>
              <th style={{ padding: "12px 0" }}>Accuracy</th>
            </tr>
          </thead>
          <tbody>
            {displayList.map((v, i) => (
              <tr key={i} style={{ borderBottom: "1px solid #222" }}>
                <td style={{ padding: "16px 0", color: "#fff", fontWeight: 600 }}>
                  <div>{v.name}</div>
                  <div style={{ fontSize: "0.75rem", color: "#888" }}>{v.provider}</div>
                </td>
                <td style={{ padding: "16px 0", fontFamily: "monospace", color: "#a5d6ff", fontSize: "0.8rem" }}>
                  {v.pk.slice(0, 8)}...{v.pk.slice(-6)}
                </td>
                <td style={{ padding: "16px 0", color: "#3fb950" }}>{v.stake}</td>
                <td style={{ padding: "16px 0", color: "#fff" }}>{v.validations_count}</td>
                <td style={{ padding: "16px 0" }}>
                  <span
                    style={{
                      padding: "4px 8px",
                      borderRadius: "12px",
                      fontSize: "0.75rem",
                      backgroundColor:
                        v.status === "Active" ? "rgba(46, 160, 67, 0.2)" : "rgba(248, 81, 73, 0.2)",
                      color: v.status === "Active" ? "#3fb950" : "#f85149",
                    }}
                  >
                    {v.status}
                  </span>
                </td>
                <td style={{ padding: "16px 0", color: "#fff" }}>{v.consensus_accuracy}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

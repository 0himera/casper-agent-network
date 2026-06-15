"use client";

import { useState } from "react";
import type { AgentEntity } from "@/entities/agent/types/types";
import styles from "./MyAgent.module.css";

interface PriceConfigProps { agent: AgentEntity }

export function PriceConfig({ agent }: PriceConfigProps) {
  const [newPrice, setNewPrice] = useState(String(agent.customPrice));
  const handleUpdate = () => alert(`Price updated to ${newPrice} CSPR (simulated)`);

  return (
    <div className={styles.section}>
      <h3 className={styles.sectionTitle}>Custom Price Config</h3>
      <div className={styles.priceRow}>
        <span className={styles.priceLabel}>Recommended Price:</span>
        <span className={styles.priceValue}>{agent.recommendedPrice} CSPR</span>
      </div>
      <div className={styles.priceRow}>
        <span className={styles.priceLabel}>Current Custom Price:</span>
        <span className={styles.priceValue}>{agent.customPrice} CSPR</span>
      </div>
      <div className={styles.priceRow}>
        <span className={styles.priceLabel}>Update Price (CSPR):</span>
        <input type="number" className={styles.priceInput} value={newPrice} onChange={(e) => setNewPrice(e.target.value)} step="0.1" min="0" />
        <button className={styles.updateButton} onClick={handleUpdate}>Update On-chain</button>
      </div>
    </div>
  );
}

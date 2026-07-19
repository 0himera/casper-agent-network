"use client";

import { useState } from "react";
import type { AgentEntity } from "@/entities/agent/types/types";
import { buildSetPriceTx } from "@/shared/utils/contract-transactions";
import { signAndSendTransaction } from "@/features/wallet/utils/signing";
import { apiPatch } from "@/shared/api/api-client";
import { useAppStore } from "@/shared/providers/AppStoreProvider";
import { toast } from "@/shared/ui/Toast";
import styles from "./MyAgent.module.css";

interface PriceConfigProps {
  agent: AgentEntity;
}

export function PriceConfig({ agent }: PriceConfigProps) {
  const walletAddress = useAppStore((s) => s.walletAddress);
  const [newPrice, setNewPrice] = useState(String(agent.customPrice));
  const [loading, setLoading] = useState(false);
  const [status, setStatus] = useState("");

  const handleUpdate = async () => {
    if (!walletAddress) {
      toast.error("Please connect your wallet first.");
      return;
    }

    setLoading(true);
    setStatus("Building transaction...");
    try {
      const priceMotes = String(Math.round(parseFloat(newPrice) * 1_000_000_000));
      const transaction = await buildSetPriceTx(walletAddress, priceMotes);

      setStatus("Signing transaction...");
      const txHash = await signAndSendTransaction(transaction, walletAddress);

      setStatus("Synchronizing price off-chain...");
      await apiPatch(`/api/agents/${agent.publicKey}/price`, {
        custom_price_motes: parseInt(priceMotes, 10),
      });

      setStatus("Price updated successfully!");
      toast.success(`On-chain price updated successfully!\nTransaction Hash: ${txHash}`);
    } catch (err: unknown) {
      console.error(err);
      setStatus("");
      toast.error(`Failed to update price: ${String(err)}`);
    } finally {
      setLoading(false);
    }
  };

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
        <input
          type="number"
          className={styles.priceInput}
          value={newPrice}
          onChange={(e) => setNewPrice(e.target.value)}
          step="0.1"
          min="0"
          disabled={loading}
        />
        <button className={styles.updateButton} onClick={handleUpdate} disabled={loading}>
          {loading ? "Updating..." : "Update On-chain"}
        </button>
      </div>
      {status && (
        <div
          className={styles.statusText}
          aria-live="polite"
          aria-atomic="true"
          style={{
            marginTop: "10px",
            fontSize: "0.85rem",
            color: "var(--text-muted)",
            opacity: 0.8,
          }}
        >
          {status}
        </div>
      )}
    </div>
  );
}

"use client";

import { useState } from "react";
import { Coins, ShieldAlert, ArrowUpRight, ArrowDownLeft, KeyRound } from "lucide-react";
import type { AgentEntity } from "@/entities/agent/types/types";
import {
  buildStakeTx,
  buildRequestUnstakeTx,
  buildSetDelegatedSignerTx,
} from "@/shared/utils/contract-transactions";
import { signAndSendTransaction } from "@/features/wallet/utils/signing";
import { useAppStore } from "@/shared/providers/AppStoreProvider";
import { toast } from "@/shared/ui/Toast";
import styles from "./MyAgent.module.css";

interface AgentStakingPanelProps {
  agent: AgentEntity;
}

export function AgentStakingPanel({ agent }: AgentStakingPanelProps) {
  const walletAddress = useAppStore((s) => s.walletAddress);
  const [stakeAmount, setStakeAmount] = useState("50");
  const [unstakeAmount, setUnstakeAmount] = useState("10");
  const [loading, setLoading] = useState(false);
  const [status, setStatus] = useState("");

  const handleDepositStake = async () => {
    if (!walletAddress) {
      toast.error("Please connect your wallet first.");
      return;
    }

    const amountNum = parseFloat(stakeAmount);
    if (isNaN(amountNum) || amountNum <= 0) {
      toast.error("Please enter a valid stake amount in CSPR.");
      return;
    }

    setLoading(true);
    setStatus("Building stake transaction...");
    try {
      const motes = String(Math.round(amountNum * 1_000_000_000));
      const transaction = await buildStakeTx(walletAddress, motes);

      setStatus("Signing stake transaction with Casper Wallet...");
      const txHash = await signAndSendTransaction(transaction, walletAddress);

      setStatus("Stake transaction submitted!");
      toast.success(
        `Staked ${amountNum} CSPR on-chain!\nTransaction Hash: ${txHash}\nYour agent is now eligible to receive tasks.`
      );
    } catch (err: unknown) {
      console.error(err);
      setStatus("");
      toast.error(`Failed to stake CSPR: ${String(err)}`);
    } finally {
      setLoading(false);
    }
  };

  const handleRequestUnstake = async () => {
    if (!walletAddress) {
      toast.error("Please connect your wallet first.");
      return;
    }

    const amountNum = parseFloat(unstakeAmount);
    if (isNaN(amountNum) || amountNum <= 0) {
      toast.error("Please enter a valid unstake amount in CSPR.");
      return;
    }

    setLoading(true);
    setStatus("Building request unstake transaction...");
    try {
      const motes = String(Math.round(amountNum * 1_000_000_000));
      const transaction = await buildRequestUnstakeTx(walletAddress, motes);

      setStatus("Signing request_unstake transaction with Casper Wallet...");
      const txHash = await signAndSendTransaction(transaction, walletAddress);

      setStatus("Unstake request submitted!");
      toast.success(
        `Unstake request of ${amountNum} CSPR submitted!\nTransaction Hash: ${txHash}\nUnbonding period: 30 minutes.`
      );
    } catch (err: unknown) {
      console.error(err);
      setStatus("");
      toast.error(`Failed to request unstake: ${String(err)}`);
    } finally {
      setLoading(false);
    }
  };

  const handleDelegateSigner = async () => {
    if (!walletAddress) {
      toast.error("Please connect your wallet first.");
      return;
    }

    setLoading(true);
    setStatus("Building set_delegated_signer transaction...");
    try {
      const adminPubkey =
        process.env.NEXT_PUBLIC_ADMIN_ACCOUNT ||
        "01ac7a93e16ccf32fa9d91d387c9fb84521e23fdae8ce57263d173beafab5fc1b8";

      const transaction = await buildSetDelegatedSignerTx(walletAddress, adminPubkey);

      setStatus("Signing set_delegated_signer transaction with Casper Wallet...");
      const txHash = await signAndSendTransaction(transaction, walletAddress);

      setStatus("Delegation transaction submitted!");
      toast.success(
        `Delegated signing authorized for cluster node!\nTransaction Hash: ${txHash}\nYour hosted agent can now automatically execute assigned tasks.`
      );
    } catch (err: unknown) {
      console.error(err);
      setStatus("");
      toast.error(`Failed to delegate signing: ${String(err)}`);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className={styles.section}>
      <h3 className={styles.sectionTitle} style={{ display: "flex", alignItems: "center", gap: "8px" }}>
        <Coins size={16} style={{ color: "var(--accent-primary)" }} />
        Agent Staking & Node Cluster Delegation
      </h3>

      <div
        style={{
          display: "flex",
          gap: "10px",
          padding: "12px",
          borderRadius: "6px",
          background: "rgba(245, 158, 11, 0.05)",
          border: "1px solid rgba(245, 158, 11, 0.2)",
          alignItems: "flex-start",
          fontSize: "12px",
          marginBottom: "16px",
        }}
      >
        <ShieldAlert size={16} style={{ color: "#f59e0b", flexShrink: 0, marginTop: "2px" }} />
        <div style={{ color: "var(--text-secondary)", lineHeight: 1.4 }}>
          <strong>Required Minimum Stake: 50 CSPR + Cluster Delegation.</strong> Smart contract verification requires a minimum 50 CSPR stake (`stake`) and node cluster signing authorization (`set_delegated_signer`) for hosted agents to accept and automatically execute tasks.
        </div>
      </div>

      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: "16px" }}>
        {/* Deposit Stake Box */}
        <div
          style={{
            background: "rgba(255,255,255,0.02)",
            border: "1px solid var(--border-color)",
            padding: "16px",
            borderRadius: "8px",
            display: "flex",
            flexDirection: "column",
            gap: "12px",
          }}
        >
          <div style={{ fontWeight: 600, fontSize: "13px", display: "flex", alignItems: "center", gap: "6px" }}>
            <ArrowUpRight size={14} style={{ color: "#10b981" }} />
            Deposit On-Chain Stake
          </div>
          <div style={{ fontSize: "12px", color: "var(--text-muted)" }}>
            Attach CSPR to your agent&apos;s smart contract profile.
          </div>
          <div style={{ display: "flex", gap: "8px", alignItems: "center" }}>
            <input
              type="number"
              className={styles.priceInput}
              value={stakeAmount}
              onChange={(e) => setStakeAmount(e.target.value)}
              step="5"
              min="1"
              disabled={loading}
              placeholder="50"
              style={{ width: "100%" }}
            />
            <span style={{ fontSize: "12px", color: "var(--text-muted)" }}>CSPR</span>
          </div>
          <button
            className={styles.updateButton}
            onClick={handleDepositStake}
            disabled={loading}
            style={{ width: "100%", justifyContent: "center" }}
          >
            {loading ? "Processing..." : "Deposit 50 CSPR Stake"}
          </button>
        </div>

        {/* Delegate Signer Box */}
        <div
          style={{
            background: "rgba(255,255,255,0.02)",
            border: "1px solid var(--border-color)",
            padding: "16px",
            borderRadius: "8px",
            display: "flex",
            flexDirection: "column",
            gap: "12px",
          }}
        >
          <div style={{ fontWeight: 600, fontSize: "13px", display: "flex", alignItems: "center", gap: "6px" }}>
            <KeyRound size={14} style={{ color: "#00f2fe" }} />
            Authorize Node Signer
          </div>
          <div style={{ fontSize: "12px", color: "var(--text-muted)" }}>
            Authorize CAN cluster node to execute jobs for this agent.
          </div>
          <div style={{ flex: 1 }} />
          <button
            className={styles.updateButton}
            onClick={handleDelegateSigner}
            disabled={loading}
            style={{
              width: "100%",
              justifyContent: "center",
              background: "rgba(0, 242, 254, 0.15)",
              color: "#00f2fe",
              border: "1px solid rgba(0, 242, 254, 0.3)",
            }}
          >
            {loading ? "Processing..." : "Authorize Delegated Signer"}
          </button>
        </div>

        {/* Request Unstake Box */}
        <div
          style={{
            background: "rgba(255,255,255,0.02)",
            border: "1px solid var(--border-color)",
            padding: "16px",
            borderRadius: "8px",
            display: "flex",
            flexDirection: "column",
            gap: "12px",
          }}
        >
          <div style={{ fontWeight: 600, fontSize: "13px", display: "flex", alignItems: "center", gap: "6px" }}>
            <ArrowDownLeft size={14} style={{ color: "#eb72ff" }} />
            Request Unstake
          </div>
          <div style={{ fontSize: "12px", color: "var(--text-muted)" }}>
            Initiate 30-minute unbonding queue for staked CSPR.
          </div>
          <div style={{ display: "flex", gap: "8px", alignItems: "center" }}>
            <input
              type="number"
              className={styles.priceInput}
              value={unstakeAmount}
              onChange={(e) => setUnstakeAmount(e.target.value)}
              step="5"
              min="1"
              disabled={loading}
              placeholder="10"
              style={{ width: "100%" }}
            />
            <span style={{ fontSize: "12px", color: "var(--text-muted)" }}>CSPR</span>
          </div>
          <button
            className={styles.updateButton}
            onClick={handleRequestUnstake}
            disabled={loading}
            style={{
              width: "100%",
              justifyContent: "center",
              background: "rgba(235, 114, 255, 0.15)",
              color: "#eb72ff",
              border: "1px solid rgba(235, 114, 255, 0.3)",
            }}
          >
            {loading ? "Processing..." : "Request Unstake"}
          </button>
        </div>
      </div>

      {status && (
        <div
          aria-live="polite"
          aria-atomic="true"
          style={{
            marginTop: "12px",
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


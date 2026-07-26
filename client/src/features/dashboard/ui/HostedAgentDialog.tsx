"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { Cpu, Check, ShieldCheck, Zap, Lock } from "lucide-react";
import { motion } from "motion/react";
import { useAppStore } from "@/shared/providers/AppStoreProvider";
import { toast } from "@/shared/ui/Toast";
import { SkillsPicker } from "@/features/agents/ui/SkillsPicker";
import {
  buildRegisterAgentTx,
  buildNativeTransferTx,
  buildSetDelegatedSignerTx,
} from "@/shared/utils/contract-transactions";
import { signAndSendTransaction } from "@/features/wallet/utils/signing";
import { apiPost } from "@/shared/api/api-client";
import type { AgentSkill } from "@/entities/agent/types/types";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/shared/ui/dialog";
import registerStyles from "@/features/agents/ui/Register.module.css";
import styles from "./HostedAgentDialog.module.css";

const features = [
  "24/7 Managed Uptime in CAN Enterprise Node Cluster",
  "Isolated Data & Memory Processing (Zero Data Leakage)",
  "Delegated On-Chain Signing for Instant Task Execution",
  "Automated Multi-Validator Audit & Reputation Tracking",
];

export function HostedAgentDialog() {
  const router = useRouter();
  const walletAddress = useAppStore((s) => s.walletAddress);

  const [open, setOpen] = useState(false);
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [skills, setSkills] = useState<AgentSkill[]>([]);
  const [systemPrompt, setSystemPrompt] = useState("");
  const [loading, setLoading] = useState(false);
  const [status, setStatus] = useState("");

  const canSubmit = walletAddress && name.trim() && !loading;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!walletAddress) {
      toast.error("Please connect your Casper Wallet first.");
      return;
    }
    if (!name.trim()) {
      toast.error("Please enter a name for your agent.");
      return;
    }

    setLoading(true);
    setStatus("Initiating 100 CSPR registration payment...");
    try {
      const adminPubkey =
        process.env.NEXT_PUBLIC_ADMIN_ACCOUNT ||
        "01ac7a93e16ccf32fa9d91d387c9fb84521e23fdae8ce57263d173beafab5fc1b8";

      // 100 CSPR = 100,000,000,000 motes
      const transferTx = buildNativeTransferTx(walletAddress, adminPubkey, "100000000000");
      setStatus("Signing 100 CSPR subscription payment...");
      const transferTxHash = await signAndSendTransaction(transferTx, walletAddress);

      const paymentObj = {
        x402Version: 1,
        scheme: "exact",
        network: "casper",
        payload: {
          paymentType: "native",
          txid: transferTxHash,
        },
      };
      const xPaymentVal = Array.from(new TextEncoder().encode(JSON.stringify(paymentObj)))
        .map((b) => b.toString(16).padStart(2, "0"))
        .join("");

      setStatus("Signing register_agent contract transaction...");
      const registerTx = await buildRegisterAgentTx(
        walletAddress,
        name,
        description || "CAN Enterprise Hosted Agent",
        "https://casper-agent-network.vercel.app/metadata/" + walletAddress,
      );
      const registerTxHash = await signAndSendTransaction(registerTx, walletAddress);

      setStatus("Signing set_delegated_signer contract transaction...");
      const setDelegatedTx = await buildSetDelegatedSignerTx(walletAddress, adminPubkey);
      const delegatedSignerTxHash = await signAndSendTransaction(setDelegatedTx, walletAddress);

      setStatus("Activating hosted agent instance...");
      await apiPost(
        "/api/agents/register",
        {
          public_key: walletAddress,
          name,
          description: description || null,
          metadata_uri: "https://casper-agent-network.vercel.app/metadata/" + walletAddress,
          endpoint_url: "http://localhost:11434",
          api_key: null,
          model: "gemma3:4b",
          system_prompt: systemPrompt.trim() || null,
          skills,
        },
        {
          "X-Payment": xPaymentVal,
        },
      );

      setStatus("Hosted Agent successfully activated!");
      toast.success(
        `Hosted agent deployed to cluster!\nRegister Tx: ${registerTxHash}\nDelegation Tx: ${delegatedSignerTxHash}`,
      );
      setOpen(false);
      router.push("/my-agent");
    } catch (err: unknown) {
      console.error(err);
      setStatus("");
      toast.error(`Registration failed: ${String(err)}`);
    } finally {
      setLoading(false);
    }
  };

  const handleOpenChange = (nextOpen: boolean) => {
    setOpen(nextOpen);
    if (!nextOpen) {
      setStatus("");
    }
  };

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <button type="button" className={styles.actionCard} onClick={() => setOpen(true)}>
        <Cpu size={18} className={styles.actionIcon} aria-hidden="true" />
        <span className={styles.actionContent}>
          <span className={styles.actionTitle}>Register Custodial Agent</span>
          <span className={styles.actionDesc}>Managed Cluster Instance</span>
        </span>
      </button>

      <DialogContent className={styles.dialogContent} style={{ maxWidth: "560px" }}>
        <div className={styles.card}>
          <div className={styles.planBadge}>[MANAGED_CLUSTER_INSTANCE]</div>
          
          <div className={styles.planHeader}>
            <DialogTitle className={styles.planTitle}>
              Custodial AI Agent Instance
            </DialogTitle>
            <DialogDescription className={styles.planDescription}>
              Dedicated AI Agent deployed inside CAN’s private enterprise cluster. Fully managed execution with zero server setup.
            </DialogDescription>
            <div className={styles.planPriceTag}>
              <span className={styles.priceValue}>100 CSPR</span>
              <span className={styles.pricePeriod}>/ month</span>
            </div>
          </div>

          <ul className={styles.features}>
            {features.map((feature) => (
              <li key={feature} className={styles.feature}>
                <Check size={14} className={styles.featureIcon} aria-hidden="true" />
                <span>{feature}</span>
              </li>
            ))}
          </ul>

          <div className={styles.clusterInfoBox}>
            <Lock size={14} className={styles.infoIcon} />
            <span>Hosted in CAN Secure Cluster &bull; Encrypted On-Chain State</span>
          </div>

          <form className={styles.form} onSubmit={handleSubmit}>
            <div className={registerStyles.field}>
              <label className={registerStyles.label}>Agent Name</label>
              <input
                className={registerStyles.input}
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="e.g. DeFi Security Auditor"
                disabled={loading}
              />
            </div>

            <div className={registerStyles.field}>
              <label className={registerStyles.label}>Description & Specialization</label>
              <textarea
                className={registerStyles.textarea}
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                placeholder="Briefly describe what your AI agent specializes in..."
                disabled={loading}
                rows={2}
              />
            </div>

            <SkillsPicker selected={skills} onChange={setSkills} />

            <div className={registerStyles.field}>
              <label className={registerStyles.label}>System Instructions & Behavior</label>
              <textarea
                className={registerStyles.textarea}
                value={systemPrompt}
                onChange={(e) => setSystemPrompt(e.target.value)}
                placeholder="Define rules, tone, and operational guidelines for this agent..."
                disabled={loading}
                rows={3}
              />
            </div>

            {status && (
              <div className={styles.statusMessage} aria-live="polite" aria-atomic="true">
                {status}
              </div>
            )}

            <motion.button
              whileHover={{ scale: canSubmit ? 1.01 : 1 }}
              whileTap={{ scale: canSubmit ? 0.99 : 1 }}
              type="submit"
              className={styles.submitButton}
              disabled={!canSubmit}
            >
              {loading ? "Deploying Agent..." : "Deploy Custodial Agent (100 CSPR)"}
            </motion.button>
          </form>
        </div>
      </DialogContent>
    </Dialog>
  );
}

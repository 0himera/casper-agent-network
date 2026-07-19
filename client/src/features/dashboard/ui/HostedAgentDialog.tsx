"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { Cloud, Check, Sparkles } from "lucide-react";
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
  "Cloud-managed agent with delegated signing",
  "24/7 uptime without running your own node",
  "Connect any OpenAI-compatible API endpoint",
  "On-chain reputation & escrow participation",
];

export function HostedAgentDialog() {
  const router = useRouter();
  const walletAddress = useAppStore((s) => s.walletAddress);

  const [open, setOpen] = useState(false);
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [skills, setSkills] = useState<AgentSkill[]>([]);
  const [endpoint, setEndpoint] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [model, setModel] = useState("");
  const [systemPrompt, setSystemPrompt] = useState("");
  const [loading, setLoading] = useState(false);
  const [status, setStatus] = useState("");

  const canSubmit = walletAddress && name.trim() && endpoint.trim() && model.trim() && !loading;

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
    if (!endpoint.trim()) {
      toast.error("Please enter an endpoint URL.");
      return;
    }
    if (!model.trim()) {
      toast.error("Please enter a model ID.");
      return;
    }

    setLoading(true);
    setStatus("Initiating 0.1 CSPR registration payment...");
    try {
      const adminPubkey =
        process.env.NEXT_PUBLIC_ADMIN_ACCOUNT ||
        "01ac7a93e16ccf32fa9d91d387c9fb84521e23fdae8ce57263d173beafab5fc1b8";

      const transferTx = buildNativeTransferTx(walletAddress, adminPubkey, "100000000");
      setStatus("Signing 0.1 CSPR payment...");
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
        description || "Casper Hosted Agent",
        "https://agentnetwork.io/metadata/" + walletAddress,
      );
      const registerTxHash = await signAndSendTransaction(registerTx, walletAddress);

      setStatus("Signing set_delegated_signer contract transaction...");
      const setDelegatedTx = await buildSetDelegatedSignerTx(walletAddress, adminPubkey);
      const delegatedSignerTxHash = await signAndSendTransaction(setDelegatedTx, walletAddress);

      setStatus("Saving hosted agent configuration...");
      await apiPost(
        "/api/agents/register",
        {
          public_key: walletAddress,
          name,
          description: description || null,
          metadata_uri: "https://agentnetwork.io/metadata/" + walletAddress,
          endpoint_url: endpoint,
          api_key: apiKey || null,
          model,
          system_prompt: systemPrompt.trim() || null,
          skills,
        },
        {
          "X-Payment": xPaymentVal,
        },
      );

      setStatus("Agent successfully registered!");
      toast.success(
        `Hosted agent registered!\nRegister Tx: ${registerTxHash}\nDelegation Tx: ${delegatedSignerTxHash}`,
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
        <Cloud size={18} className={styles.actionIcon} aria-hidden="true" />
        <span className={styles.actionContent}>
          <span className={styles.actionTitle}>Register Hosted Agent</span>
          <span className={styles.actionDesc}>Cloud-managed, no server setup</span>
        </span>
      </button>

      <DialogContent className={styles.dialogContent} style={{ maxWidth: "540px" }}>
        <div className={styles.card}>
          <div className={styles.planHeader}>
            <div className={styles.planIcon}>
              <Cloud size={28} aria-hidden="true" />
            </div>
            <DialogTitle className={styles.planTitle}>
              Hosted Agent
              <Sparkles size={14} className={styles.planBadgeIcon} aria-hidden="true" />
            </DialogTitle>
            <DialogDescription className={styles.planDescription}>
              Cloud-managed agent with delegated signing
            </DialogDescription>
            <div className={styles.planPrice}>
              <span className={styles.priceValue}>0.1 CSPR</span>
              <span className={styles.pricePeriod}>one-time registration</span>
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

          <form className={styles.form} onSubmit={handleSubmit}>
            <div className={registerStyles.field}>
              <label className={registerStyles.label}>Agent Name</label>
              <input
                className={registerStyles.input}
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="My DeFi Bot"
                disabled={loading}
              />
            </div>

            <div className={registerStyles.field}>
              <label className={registerStyles.label}>Description</label>
              <textarea
                className={registerStyles.textarea}
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                placeholder="What does your agent do?"
                disabled={loading}
                rows={2}
              />
            </div>

            <SkillsPicker selected={skills} onChange={setSkills} />

            <div className={styles.hostedFields}>
              <div className={registerStyles.field}>
                <label className={registerStyles.label}>Endpoint URL</label>
                <input
                  className={registerStyles.input}
                  value={endpoint}
                  onChange={(e) => setEndpoint(e.target.value)}
                  placeholder="https://api.example.com/v1/chat/completions"
                  disabled={loading}
                />
              </div>

              <div className={registerStyles.field}>
                <label className={registerStyles.label}>API Key</label>
                <input
                  className={registerStyles.input}
                  type="password"
                  value={apiKey}
                  onChange={(e) => setApiKey(e.target.value)}
                  placeholder="sk-..."
                  disabled={loading}
                />
              </div>

              <div className={registerStyles.field}>
                <label className={registerStyles.label}>Model ID</label>
                <input
                  className={registerStyles.input}
                  value={model}
                  onChange={(e) => setModel(e.target.value)}
                  placeholder="gpt-4o-mini"
                  disabled={loading}
                />
              </div>

              <div className={registerStyles.field}>
                <label className={registerStyles.label}>System Prompt</label>
                <textarea
                  className={registerStyles.textarea}
                  value={systemPrompt}
                  onChange={(e) => setSystemPrompt(e.target.value)}
                  placeholder="Instructions for agent behavior"
                  disabled={loading}
                  rows={2}
                />
              </div>
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
              {loading ? "Processing..." : "Subscribe & Register"}
            </motion.button>
          </form>
        </div>
      </DialogContent>
    </Dialog>
  );
}

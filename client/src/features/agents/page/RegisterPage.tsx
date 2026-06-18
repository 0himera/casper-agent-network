"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { Wrench } from "lucide-react";
import type { AgentSkill, AgentExecutionMode } from "@/entities/agent/types/types";
import { SkillsPicker } from "@/features/agents/ui/SkillsPicker";
import { AgentTypePicker } from "@/features/agents/ui/AgentTypePicker";
import { motion } from "motion/react";
import { buildRegisterAgentTx, buildNativeTransferTx } from "@/shared/utils/contract-transactions";
import { signAndSendTransaction } from "@/features/wallet/utils/signing";
import { apiPost } from "@/shared/api/api-client";
import { useAppStore } from "@/shared/providers/AppStoreProvider";
import styles from "@/features/agents/ui/Register.module.css";

export default function RegisterPage() {
  const router = useRouter();
  const walletAddress = useAppStore((s) => s.walletAddress);

  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [skills, setSkills] = useState<AgentSkill[]>([]);
  const [agentType, setAgentType] = useState<AgentExecutionMode>("hosted");
  const [endpoint, setEndpoint] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [model, setModel] = useState("");
  const [systemPrompt, setSystemPrompt] = useState("");
  const [loading, setLoading] = useState(false);
  const [status, setStatus] = useState("");

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!walletAddress) {
      alert("Please connect your Casper Wallet using the button in the top bar.");
      return;
    }

    if (!name.trim()) {
      alert("Please enter a name for your agent.");
      return;
    }

    setLoading(true);
    setStatus("Initiating 0.1 CSPR registration payment...");
    try {
      const adminPubkey = process.env.NEXT_PUBLIC_ADMIN_ACCOUNT || "01ac7a93e16ccf32fa9d91d387c9fb84521e23fdae8ce57263d173beafab5fc1b8";

      const transferTx = buildNativeTransferTx(walletAddress, adminPubkey, "100000000");
      setStatus("Signing 0.1 CSPR payment...");
      const transferTxHash = await signAndSendTransaction(transferTx, walletAddress);

      const paymentObj = {
        x402Version: 1,
        scheme: "exact",
        network: "casper",
        payload: {
          paymentType: "native",
          txid: transferTxHash
        }
      };
      const xPaymentVal = Array.from(new TextEncoder().encode(JSON.stringify(paymentObj)))
        .map(b => b.toString(16).padStart(2, '0'))
        .join('');

      setStatus("Signing register_agent contract transaction...");
      const registerTx = await buildRegisterAgentTx(
        walletAddress,
        name,
        description || "Casper Autonomous Agent",
        "https://agentnetwork.io/metadata/" + walletAddress
      );
      const registerTxHash = await signAndSendTransaction(registerTx, walletAddress);

      setStatus("Saving bot configuration off-chain...");
      await apiPost("/api/agents/register", {
        public_key: walletAddress,
        name,
        description: description || null,
        metadata_uri: "https://agentnetwork.io/metadata/" + walletAddress,
        endpoint_url: agentType === "hosted" ? null : endpoint,
        api_key: agentType === "hosted" ? null : apiKey,
        model: agentType === "hosted" ? null : model,
        system_prompt: systemPrompt.trim() || null,
        skills: skills
      }, {
        "X-Payment": xPaymentVal
      });

      setStatus("Agent successfully registered!");
      alert(`Bot registered on-chain and off-chain!\nRegister Tx: ${registerTxHash}`);
      router.push("/my-agent");
    } catch (err: any) {
      console.error(err);
      setStatus("");
      alert(`Registration failed: ${err.message || err}`);
    } finally {
      setLoading(false);
    }
  };

  return (
    <motion.div
      className={styles.page}
      initial={{ opacity: 0, y: 15 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.35, ease: "easeOut" }}
    >
      <h1 className={styles.title}><Wrench size={20} /> Register Bot</h1>
      <form className={styles.form} onSubmit={handleSubmit}>
        <div className={styles.field}>
          <label className={styles.label}>Agent Name</label>
          <input className={styles.input} value={name} onChange={(e) => setName(e.target.value)} placeholder="My DeFi Bot" disabled={loading} />
        </div>
        <div className={styles.field}>
          <label className={styles.label}>Description</label>
          <textarea className={styles.textarea} value={description} onChange={(e) => setDescription(e.target.value)} placeholder="What does your agent do?" disabled={loading} />
        </div>
        <SkillsPicker selected={skills} onChange={setSkills} />
        <AgentTypePicker type={agentType} onChange={setAgentType} endpoint={endpoint} apiKey={apiKey} model={model} systemPrompt={systemPrompt} onEndpointChange={setEndpoint} onApiKeyChange={setApiKey} onModelChange={setModel} onSystemPromptChange={setSystemPrompt} />

        {status && (
          <div className={styles.statusMessage} style={{ padding: "12px", background: "rgba(255,255,255,0.05)", borderRadius: "8px", border: "1px solid rgba(255,255,255,0.1)", fontSize: "0.9rem", color: "var(--text-muted)", marginBottom: "16px" }}>
            {status}
          </div>
        )}

        <motion.button
          whileHover={{ scale: loading ? 1 : 1.01 }}
          whileTap={{ scale: loading ? 1 : 0.99 }}
          type="submit"
          className={styles.submitButton}
          disabled={loading}
        >
          {loading ? "Processing..." : "Sign & Register Agent On-Chain"}
        </motion.button>
      </form>
    </motion.div>
  );
}

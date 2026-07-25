"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { Cpu, Check, ShieldCheck, Lock, Sparkles } from "lucide-react";
import type { AgentSkill } from "@/entities/agent/types/types";
import { SkillsPicker } from "@/features/agents/ui/SkillsPicker";
import { motion } from "motion/react";
import {
  buildRegisterAgentTx,
  buildNativeTransferTx,
  buildSetDelegatedSignerTx,
  buildStakeTx,
} from "@/shared/utils/contract-transactions";
import { signAndSendTransaction } from "@/features/wallet/utils/signing";
import { apiPost } from "@/shared/api/api-client";
import { useAppStore } from "@/shared/providers/AppStoreProvider";
import { toast } from "@/shared/ui/Toast";
import styles from "@/features/agents/ui/Register.module.css";

const features = [
  "24/7 Managed Uptime in CAN Enterprise Node Cluster",
  "Isolated Data & Memory Processing (Zero Data Leakage)",
  "Delegated On-Chain Signing for Instant Task Execution",
  "Automated Multi-Validator Audit & Reputation Tracking",
  "50 CSPR Minimum On-Chain Stake Deposited to Smart Contract",
];

export default function RegisterPage() {
  const router = useRouter();
  const walletAddress = useAppStore((s) => s.walletAddress);

  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [skills, setSkills] = useState<AgentSkill[]>([]);
  const [systemPrompt, setSystemPrompt] = useState("");
  const [loading, setLoading] = useState(false);
  const [status, setStatus] = useState("");

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!walletAddress) {
      toast.error("Please connect your Casper Wallet using the button in the top bar.");
      return;
    }

    if (!name.trim()) {
      toast.error("Please enter a name for your agent.");
      return;
    }

    setLoading(true);
    setStatus("Initiating 100 CSPR subscription payment...");
    try {
      const adminPubkey =
        process.env.NEXT_PUBLIC_ADMIN_ACCOUNT ||
        "01ac7a93e16ccf32fa9d91d387c9fb84521e23fdae8ce57263d173beafab5fc1b8";

      // 1. 100 CSPR = 100,000,000,000 motes
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

      // 2. Register Agent on-chain
      setStatus("Signing register_agent contract transaction...");
      const registerTx = await buildRegisterAgentTx(
        walletAddress,
        name,
        description || "CAN Enterprise Hosted Agent",
        "https://casper-agent-network.vercel.app/metadata/" + walletAddress,
      );
      const registerTxHash = await signAndSendTransaction(registerTx, walletAddress);

      // 3. Deposit 50 CSPR Minimum Stake to Smart Contract
      setStatus("Signing 50 CSPR agent contract stake transaction...");
      const stakeTx = await buildStakeTx(walletAddress, "50000000000");
      const stakeTxHash = await signAndSendTransaction(stakeTx, walletAddress);

      // 4. Delegate signing to cluster node
      setStatus("Signing set_delegated_signer contract transaction...");
      const setDelegatedTx = await buildSetDelegatedSignerTx(walletAddress, adminPubkey);
      const delegatedSignerTxHash = await signAndSendTransaction(setDelegatedTx, walletAddress);

      // 5. Deploy Hosted Agent Instance
      setStatus("Deploying hosted agent instance to CAN cluster...");
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
          skills: skills,
        },
        {
          "X-Payment": xPaymentVal,
        },
      );

      setStatus("Hosted Agent successfully activated & staked!");
      toast.success(
        `Hosted agent deployed & staked!\nRegister Tx: ${registerTxHash}\nStake Tx: ${stakeTxHash}\nDelegation Tx: ${delegatedSignerTxHash}`,
      );
      router.push("/my-agent");
    } catch (err: unknown) {
      console.error(err);
      setStatus("");
      toast.error(`Registration failed: ${String(err)}`);
    } finally {
      setLoading(false);
    }
  };

  return (
    <motion.div
      className={styles.page}
      initial={{ opacity: 0, y: 12 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.35, ease: "easeOut" }}
    >
      <div className={styles.productCard}>
        <div className={styles.productBadge}>[HOSTED_NODE_CLUSTER]</div>

        <div className={styles.productHeader}>
          <h1 className={styles.productTitle}>
            <Cpu size={22} className={styles.productTitleIcon} /> Hosted AI Agent Instance
          </h1>
          <p className={styles.productSubtitle}>
            Deploy an autonomous AI agent directly into CAN’s enterprise node cluster. Fully managed 24/7 execution with zero server configuration.
          </p>
          <div className={styles.priceTag}>
            <span className={styles.priceValue}>100 CSPR</span>
            <span className={styles.pricePeriod}>/ month + 50 CSPR Stake</span>
          </div>
        </div>

        <ul className={styles.featureList}>
          {features.map((f) => (
            <li key={f} className={styles.featureItem}>
              <Check size={14} className={styles.featureCheck} />
              <span>{f}</span>
            </li>
          ))}
        </ul>

        <div className={styles.guaranteeBox}>
          <Lock size={14} className={styles.guaranteeIcon} />
          <span>Hosted in CAN Secure Cluster &bull; Zero Data Leakage &bull; 50 CSPR Staked</span>
        </div>

        <form className={styles.form} onSubmit={handleSubmit}>
          <div className={styles.field}>
            <label className={styles.label}>Agent Name</label>
            <input
              className={styles.input}
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="e.g. DeFi Trading Advisor"
              disabled={loading}
            />
          </div>

          <div className={styles.field}>
            <label className={styles.label}>Description & Specialization</label>
            <textarea
              className={styles.textarea}
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="Describe your agent's primary domain and capabilities..."
              disabled={loading}
              rows={2}
            />
          </div>

          <SkillsPicker selected={skills} onChange={setSkills} />

          <div className={styles.field}>
            <label className={styles.label}>System Instructions & Behavioral Rules</label>
            <textarea
              className={styles.textarea}
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
            whileHover={{ scale: loading ? 1 : 1.01 }}
            whileTap={{ scale: loading ? 1 : 0.99 }}
            type="submit"
            className={styles.submitButton}
            disabled={loading}
          >
            {loading ? "Deploying Instance..." : "Buy & Deploy Hosted Agent (100 CSPR + 50 CSPR Stake)"}
          </motion.button>
        </form>
      </div>
    </motion.div>
  );
}


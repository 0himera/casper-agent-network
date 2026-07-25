"use client";

import Link from "next/link";
import { Bot, Shield, Globe, Cpu, Network, Calendar, Layers } from "lucide-react";
import { useAgentsQuery } from "@/features/agents/api/queries";
import { useAppStore } from "@/shared/providers/AppStoreProvider";
import { truncateAddress } from "@/shared/utils/format";
import { SKILL_LABELS, AgentSkill } from "@/entities/agent/types/types";
import { MyAgentStats } from "@/features/agent-profile/ui/MyAgentStats";
import { PriceConfig } from "@/features/agent-profile/ui/PriceConfig";
import { SkillBars } from "@/features/agent-profile/ui/SkillBars";
import { AgentStakingPanel } from "@/features/agent-profile/ui/AgentStakingPanel";
import { BenchmarkPanel } from "@/features/agent-profile/ui/BenchmarkPanel";
import styles from "@/features/agent-profile/ui/MyAgent.module.css";

export default function MyAgentPage() {
  const walletAddress = useAppStore((s) => s.walletAddress);
  const { data: agents } = useAgentsQuery();
  const agent = walletAddress && agents?.length
    ? agents.find((a: any) => a.publicKey.toLowerCase() === walletAddress.toLowerCase())
    : null;

  if (!agent) {
    return (
      <div className={styles.page}>
        <div className={styles.emptyState}>
          <Bot size={64} style={{ opacity: 0.3, color: "var(--text-muted)" }} />
          <div className={styles.emptyTitle}>No Agent Registered</div>
          <div className={styles.emptyDescription}>Connect your wallet and register a bot.</div>
          <Link href="/register" className={styles.registerLink}>Register Bot</Link>
        </div>
      </div>
    );
  }

  return (
    <div className={styles.page}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <h1 className={styles.title}>My Agent Dashboard</h1>
        <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
          <span style={{
            fontSize: "10px",
            textTransform: "uppercase",
            letterSpacing: "0.06em",
            padding: "4px 8px",
            borderRadius: "6px",
            background: agent.executionMode === "autonomous" ? "rgba(0, 242, 254, 0.08)" : "rgba(235, 114, 255, 0.08)",
            color: agent.executionMode === "autonomous" ? "#00f2fe" : "#eb72ff",
            border: agent.executionMode === "autonomous" ? "1px solid rgba(0, 242, 254, 0.15)" : "1px solid rgba(235, 114, 255, 0.15)",
            fontWeight: 600
          }}>
            {agent.executionMode} Execution
          </span>
        </div>
      </div>

      <div className={styles.statusRow} style={{ marginTop: "-12px", marginBottom: "8px" }}>
        <div className={styles.statusDot} style={{ background: agent.status === "active" ? "#10b981" : "#f59e0b" }} />
        <span className={styles.statusLabel} style={{ textTransform: "capitalize" }}>{agent.status}</span>
        <span className={styles.publicKey} style={{ fontSize: "12px" }}>{agent.publicKey}</span>
      </div>

      <MyAgentStats agent={agent} />

      <PriceConfig agent={agent} />

      <AgentStakingPanel agent={agent} />

      <SkillBars agent={agent} />

      {/* Network Registry & Telemetry */}
      <div className={styles.section}>
        <h3 className={styles.sectionTitle} style={{ display: "flex", alignItems: "center", gap: "8px" }}>
          <Network size={14} style={{ color: "var(--accent-primary)" }} />
          Network Registry & Telemetry
        </h3>
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "20px", fontSize: "13px", marginTop: "12px" }}>
          <div style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
            <div style={{ display: "flex", gap: "16px" }}>
              <span style={{ color: "var(--text-muted)", width: "120px", flexShrink: 0, display: "flex", alignItems: "center", gap: "6px" }}>
                <Calendar size={13} /> Registered:
              </span>
              <span style={{ color: "var(--text-primary)" }}>{new Date(agent.createdAt).toLocaleString()}</span>
            </div>
            
            <div style={{ display: "flex", gap: "16px" }}>
              <span style={{ color: "var(--text-muted)", width: "120px", flexShrink: 0, display: "flex", alignItems: "center", gap: "6px" }}>
                <Globe size={13} /> Metadata URI:
              </span>
              <a href={agent.metadataUri} target="_blank" rel="noopener noreferrer" style={{ color: "var(--accent-primary)", wordBreak: "break-all", textDecoration: "underline" }}>
                {agent.metadataUri || "N/A"}
              </a>
            </div>
          </div>

          <div style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
            <div style={{ display: "flex", gap: "16px" }}>
              <span style={{ color: "var(--text-muted)", width: "120px", flexShrink: 0, display: "flex", alignItems: "center", gap: "6px" }}>
                <Layers size={13} /> Active Jobs:
              </span>
              <span style={{ color: "var(--text-primary)", fontWeight: 600 }}>{agent.activeJobs} running</span>
            </div>

            <div style={{ display: "flex", gap: "16px" }}>
              <span style={{ color: "var(--text-muted)", width: "120px", flexShrink: 0, display: "flex", alignItems: "center", gap: "6px" }}>
                <Shield size={13} /> Status Reason:
              </span>
              <span style={{ color: "var(--text-secondary)" }}>
                {agent.status === "active" 
                  ? "Verified and successfully benchmarked on the network." 
                  : "Benchmark validation in progress."}
              </span>
            </div>
          </div>
        </div>
      </div>

      {/* Technical Configuration */}
      <div className={styles.section}>
        <h3 className={styles.sectionTitle} style={{ display: "flex", alignItems: "center", gap: "8px" }}>
          <Cpu size={14} style={{ color: "var(--accent-primary)" }} />
          LLM & Execution Details
        </h3>
        
        <div style={{ display: "flex", flexDirection: "column", gap: "16px", fontSize: "13px", marginTop: "12px" }}>
          <div style={{ display: "flex", gap: "16px" }}>
            <span style={{ color: "var(--text-muted)", width: "120px", flexShrink: 0 }}>Description:</span>
            <span style={{ color: "var(--text-primary)", lineHeight: "1.4" }}>
              {agent.description || "No description provided."}
            </span>
          </div>

          <div style={{ display: "flex", gap: "16px" }}>
            <span style={{ color: "var(--text-muted)", width: "120px", flexShrink: 0 }}>Model ID:</span>
            <span style={{ color: "var(--text-primary)", fontFamily: "var(--font-mono)", fontSize: "12px" }}>
              {agent.model || "N/A (Standard autonomous)"}
            </span>
          </div>

          <div style={{ display: "flex", gap: "16px" }}>
            <span style={{ color: "var(--text-muted)", width: "120px", flexShrink: 0 }}>Endpoint URL:</span>
            <span style={{ color: "var(--text-primary)", fontFamily: "var(--font-mono)", fontSize: "12px", wordBreak: "break-all" }}>
              {agent.endpointUrl || "Self-hosted (Autonomous polling)"}
            </span>
          </div>

          {agent.executionMode === "hosted" ? (
            <div style={{ 
              display: "flex", 
              gap: "10px", 
              padding: "12px", 
              borderRadius: "6px", 
              background: "rgba(0, 242, 254, 0.03)", 
              border: "1px solid rgba(0, 242, 254, 0.1)",
              alignItems: "flex-start",
              fontSize: "12px"
            }}>
              <Shield size={16} style={{ color: "#00f2fe", flexShrink: 0, marginTop: "1px" }} />
              <div style={{ color: "var(--text-secondary)" }}>
                <strong>Secure Credential Storage:</strong> Your agent endpoint credentials (API key) are securely encrypted and stored on the network coordinator. They are never exposed to public requests or client-side code.
              </div>
            </div>
          ) : (
            <div style={{ 
              display: "flex", 
              gap: "10px", 
              padding: "12px", 
              borderRadius: "6px", 
              background: "rgba(235, 114, 255, 0.03)", 
              border: "1px solid rgba(235, 114, 255, 0.1)",
              alignItems: "flex-start",
              fontSize: "12px"
            }}>
              <Shield size={16} style={{ color: "#eb72ff", flexShrink: 0, marginTop: "1px" }} />
              <div style={{ color: "var(--text-secondary)" }}>
                <strong>Autonomous Mode Guidance:</strong> This agent is running locally on your own infrastructure. The coordinator assigns tasks based on the agent's active subscriptions and on-chain registrations. You must keep your local runner daemon active to receive and sign jobs.
              </div>
            </div>
          )}

          {agent.systemPrompt && (
            <div style={{ display: "flex", flexDirection: "column", gap: "6px", marginTop: "4px" }}>
              <span style={{ color: "var(--text-muted)" }}>System Prompt:</span>
              <pre style={{
                background: "rgba(0,0,0,0.15)",
                border: "1px solid var(--border-color)",
                padding: "12px",
                borderRadius: "6px",
                fontFamily: "var(--font-mono)",
                fontSize: "12px",
                whiteSpace: "pre-wrap",
                color: "var(--text-secondary)",
                lineHeight: 1.5
              }}>{agent.systemPrompt}</pre>
            </div>
          )}
        </div>
      </div>

      <BenchmarkPanel publicKey={agent.publicKey} />
    </div>
  );
}

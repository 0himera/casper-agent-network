import { Cloud, Monitor } from "lucide-react";
import type { AgentEntity } from "@/entities/agent/types/types";
import styles from "./AgentDetail.module.css";

interface AgentTechInfoProps { agent: AgentEntity }

export function AgentTechInfo({ agent }: AgentTechInfoProps) {
  return (
    <div className={styles.section}>
      <h3 className={styles.sectionTitle}>Agent Info</h3>
      <div className={styles.infoGrid}>
        <span className={styles.infoLabel}>Model</span>
        <span className={styles.infoValue}>{agent.model ?? "N/A"}</span>
        <span className={styles.infoLabel}>Endpoint</span>
        <span className={styles.infoValue}>{agent.endpointUrl ?? "Self-hosted"}</span>
        <span className={styles.infoLabel}>Mode</span>
        <span className={styles.infoValue}>{agent.executionMode === "hosted" ? <><Cloud size={13} /> Hosted</> : <><Monitor size={13} /> Autonomous</>}</span>
        <span className={styles.infoLabel}>Price</span>
        <span className={styles.infoValue}>{agent.customPrice} CSPR (recommended: {agent.recommendedPrice} CSPR)</span>
      </div>
    </div>
  );
}

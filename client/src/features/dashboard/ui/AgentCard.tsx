import Link from "next/link";
import type { AgentEntity } from "@/entities/agent/types/types";
import { SKILL_LABELS } from "@/entities/agent/types/types";
import { StatusBadge } from "@/entities/agent/ui/StatusBadge";
import { truncateAddress, formatCSPR, getInitials, stringToColor } from "@/shared/utils/format";
import { CopyButton } from "@/shared/ui";
import { motion } from "motion/react";
import styles from "./Dashboard.module.css";

interface AgentCardProps {
  agent: AgentEntity;
}

const itemVariants = {
  hidden: { opacity: 0, y: 15, scale: 0.98 },
  show: {
    opacity: 1,
    y: 0,
    scale: 1,
    transition: {
      type: "spring",
      stiffness: 100,
      damping: 15,
    },
  },
} as const;

const MotionLink = motion.create(Link);

export function AgentCard({ agent }: AgentCardProps) {
  return (
    <MotionLink
      href={`/agents/${agent.publicKey}`}
      className={styles.agentCard}
      variants={itemVariants}
      whileHover={{
        y: -4,
        scale: 1.01,
        borderColor: "rgba(143, 174, 139, 0.4)",
        boxShadow: "0 12px 30px rgba(143, 174, 139, 0.08)",
      }}
      transition={{ duration: 0.2 }}
    >
      <div className={styles.agentCardHeader}>
        <div className={styles.agentAvatar} style={{ background: stringToColor(agent.publicKey) }}>
          {getInitials(agent.name)}
        </div>
        <div className={styles.agentInfo}>
          <div className={styles.agentName}>{agent.name}</div>
          <div className={styles.agentKey} style={{ display: "flex", alignItems: "center", gap: "4px" }}>
            {truncateAddress(agent.publicKey, 8, 6)}
            <CopyButton value={agent.publicKey} size={11} />
          </div>
        </div>
        <div style={{ display: "flex", flexDirection: "column", alignItems: "flex-end", gap: "4px" }}>
          <StatusBadge status={agent.status} />
          <span style={{
            fontSize: "9px",
            textTransform: "uppercase",
            letterSpacing: "0.05em",
            padding: "2px 6px",
            borderRadius: "4px",
            background: agent.executionMode === "autonomous" ? "rgba(0, 242, 254, 0.08)" : "rgba(235, 114, 255, 0.08)",
            color: agent.executionMode === "autonomous" ? "#00f2fe" : "#eb72ff",
            border: agent.executionMode === "autonomous" ? "1px solid rgba(0, 242, 254, 0.15)" : "1px solid rgba(235, 114, 255, 0.15)",
            fontWeight: 600
          }}>
            {agent.executionMode}
          </span>
        </div>
      </div>
      <div className={styles.agentDescription}>
        {agent.description || "An autonomous agent executing tasks and providing DeFi analysis on the Casper Agent Network."}
      </div>
      <div className={styles.agentSkills}>
        {agent.skills.map((s) => (
          <span key={s} className={styles.skillTag}>{SKILL_LABELS[s]}</span>
        ))}
      </div>
      <div className={styles.agentMeta}>
        <MetaItem label="Score" value={String(agent.reputationScore)} highlight />
        <MetaItem label="Tasks" value={String(agent.totalTasksCompleted)} />
        <MetaItem label="Price" value={`${agent.customPrice} CSPR`} />
        <MetaItem label="Earned" value={formatCSPR(agent.totalEarnings)} />
      </div>
    </MotionLink>
  );
}

function MetaItem({ label, value, highlight }: { label: string; value: string; highlight?: boolean }) {
  return (
    <div className={styles.metaItem}>
      <span className={styles.metaLabel}>{label}</span>
      <span className={`${styles.metaValue} ${highlight ? styles.scoreValue : ""}`}>{value}</span>
    </div>
  );
}

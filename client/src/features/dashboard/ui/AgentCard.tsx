import Link from "next/link";
import type { AgentEntity } from "@/entities/agent/types/types";
import { SKILL_LABELS } from "@/entities/agent/types/types";
import { StatusBadge } from "@/entities/agent/ui/StatusBadge";
import { truncateAddress, formatCSPR, getInitials, stringToColor } from "@/shared/utils/format";
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
          <div className={styles.agentKey}>{truncateAddress(agent.publicKey, 8, 6)}</div>
        </div>
        <StatusBadge status={agent.status} />
      </div>
      <div className={styles.agentDescription}>{agent.description}</div>
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

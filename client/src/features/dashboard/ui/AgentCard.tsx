import Link from "next/link";
import type { AgentEntity } from "@/entities/agent/types/types";
import { SKILL_LABELS } from "@/entities/agent/types/types";
import { StatusBadge } from "@/entities/agent/ui/StatusBadge";
import { truncateAddress, formatCSPR, getInitials, stringToColor } from "@/shared/utils/format";
import styles from "./Dashboard.module.css";

interface AgentCardProps {
  agent: AgentEntity;
}

export function AgentCard({ agent }: AgentCardProps) {
  return (
    <Link href={`/agents/${agent.publicKey}`} className={styles.agentCard}>
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
    </Link>
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

import { Copy, Bot } from "lucide-react";
import type { AgentEntity } from "@/entities/agent/types/types";
import { getInitials, stringToColor, truncateAddress } from "@/shared/utils/format";
import styles from "./AgentDetail.module.css";

interface AgentHeroProps { agent: AgentEntity }

export function AgentHero({ agent }: AgentHeroProps) {
  const handleCopy = () => navigator.clipboard.writeText(agent.publicKey);
  return (
    <div className={styles.hero}>
      <div className={styles.heroAvatar} style={{ background: stringToColor(agent.publicKey) }}>{getInitials(agent.name)}</div>
      <div className={styles.heroInfo}>
        <h1 className={styles.heroName}><Bot size={20} /> {agent.name}</h1>
        <div className={styles.heroKey}>
          {truncateAddress(agent.publicKey, 12, 8)}
          <button className={styles.copyButton} onClick={handleCopy} aria-label="Copy"><Copy size={14} /></button>
        </div>
        <div className={styles.heroDescription}>{agent.description}</div>
        <button className={styles.hireButton}>Hire Agent ({agent.recommendedPrice} CSPR)</button>
      </div>
    </div>
  );
}

import { Bot } from "lucide-react";
import type { AgentEntity } from "@/entities/agent/types/types";
import { getInitials, stringToColor, truncateAddress } from "@/shared/utils/format";
import { CopyButton } from "@/shared/ui";
import styles from "./AgentDetail.module.css";

interface AgentHeroProps { agent: AgentEntity }

export function AgentHero({ agent }: AgentHeroProps) {
  return (
    <div className={styles.hero}>
      <div className={styles.heroAvatar} style={{ background: stringToColor(agent.publicKey) }}>{getInitials(agent.name)}</div>
      <div className={styles.heroInfo}>
        <h1 className={styles.heroName}><Bot size={20} /> {agent.name}</h1>
        <div className={styles.heroKey}>
          {truncateAddress(agent.publicKey, 12, 8)}
          <CopyButton value={agent.publicKey} size={13} className="ml-1" />
        </div>
        <div className={styles.heroDescription}>{agent.description}</div>
        <button className={styles.hireButton}>Hire Agent ({agent.recommendedPrice} CSPR)</button>
      </div>
    </div>
  );
}

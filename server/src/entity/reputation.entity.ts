import { Entity, PrimaryColumn, Column, CreateDateColumn } from 'typeorm';

@Entity('reputations')
export class ReputationEntity {
  @PrimaryColumn({ type: 'varchar' })
  id: string;

  @Column({ type: 'varchar' })
  agent_public_key: string;

  @Column({ type: 'varchar' })
  skill: string;

  @Column({ type: 'int', default: 0 })
  score: number;

  @CreateDateColumn({ type: 'timestamp', default: () => 'CURRENT_TIMESTAMP' })
  timestamp: Date;
}

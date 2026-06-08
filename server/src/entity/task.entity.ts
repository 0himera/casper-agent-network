import { Entity, PrimaryColumn, Column, CreateDateColumn } from 'typeorm';

@Entity('tasks')
export class TaskEntity {
  @PrimaryColumn({ type: 'varchar' })
  id: string;

  @Column({ type: 'varchar' })
  creator_public_key: string;

  @Column({ type: 'varchar', nullable: true })
  assigned_agent_public_key?: string;

  @Column({ type: 'bigint', unsigned: true })
  budget_motes: string;

  @Column({ type: 'varchar', default: 'Open' })
  status: string;

  @Column({ type: 'varchar', nullable: true })
  result_hash?: string;

  @Column({ type: 'varchar', nullable: true })
  metadata_uri?: string;

  @Column({ type: 'varchar' })
  transaction_hash: string;

  @Column({ type: 'varchar', default: 'defi_analysis' })
  domain: string;

  @Column({ type: 'text' })
  prompt: string;

  @CreateDateColumn({ type: 'timestamp', default: () => 'CURRENT_TIMESTAMP' })
  timestamp: Date;
}

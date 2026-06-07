import { Entity, PrimaryColumn, Column, CreateDateColumn } from 'typeorm';

@Entity('agents')
export class AgentEntity {
  @PrimaryColumn({ type: 'varchar' })
  public_key: string;

  @Column({ type: 'varchar' })
  name: string;

  @Column({ type: 'text', nullable: true })
  description?: string;

  @Column({ type: 'varchar', nullable: true })
  metadata_uri?: string;

  @Column({ type: 'int', default: 0 })
  active_jobs: number;

  @CreateDateColumn({ type: 'timestamp', default: () => 'CURRENT_TIMESTAMP' })
  timestamp: Date;
}

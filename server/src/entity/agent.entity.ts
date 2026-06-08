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

  @Column({ type: 'varchar', nullable: true })
  endpoint_url?: string;

  @Column({ type: 'varchar', nullable: true })
  api_key?: string;

  @Column({ type: 'text', nullable: true })
  system_prompt?: string;

  @Column({ type: 'int', default: 0 })
  active_jobs: number;

  @Column({ type: 'varchar', default: 'active' })
  status: string;

  @Column({ type: 'bigint', unsigned: true, default: 0 })
  recommended_price_motes: string;

  @Column({ type: 'bigint', unsigned: true, default: 0 })
  custom_price_motes: string;

  @CreateDateColumn({ type: 'timestamp', default: () => 'CURRENT_TIMESTAMP' })
  timestamp: Date;
}

import { DataSource, DataSourceOptions } from 'typeorm';
import { config } from './config';
import { AgentEntity } from "./entity/agent.entity";
import { TaskEntity } from "./entity/task.entity";
import { ReputationEntity } from "./entity/reputation.entity";

export const dataSourceOptions: DataSourceOptions = {
  type: 'mysql',
  url: config.dbURI,
  synchronize: false,
  logging: false,
  supportBigNumbers: true,
  logger: 'simple-console',
  entities: [AgentEntity, TaskEntity, ReputationEntity],
};

export const AppDataSource = new DataSource(dataSourceOptions);

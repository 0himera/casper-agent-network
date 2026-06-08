import { config } from './config';
import WebSocket from 'ws';
import { AppDataSource } from "./data-source";
import { CSPRCloudAPIClient } from "./cspr-cloud/api-client";
import { formatDate } from "./utils";
import { 
  ContractEvent, 
  AgentRegisteredPayload, 
  TaskCreatedPayload, 
  TaskAssignedPayload, 
  TaskSubmittedPayload, 
  TaskCompletedPayload, 
  ScoreUpdatedPayload,
  PriceUpdatedPayload,
  RecommendedPriceUpdatedPayload
} from "./events";
import { AgentEntity } from "./entity/agent.entity";
import { TaskEntity } from "./entity/task.entity";
import { ReputationEntity } from "./entity/reputation.entity";

async function main() {
  await AppDataSource.initialize();
  console.log('Database initialized successfully.');

  const ws = new WebSocket(
      `${config.csprCloudStreamingUrl}/contract-events?contract_package_hash=${config.donationContractPackageHash}`,
      {
        headers: {
          authorization: config.csprCloudAccessKey,
        },
      },
  );

  ws.on('open', () => {
    console.log(`Connected to streaming API: ${config.csprCloudStreamingUrl}`);
  });

  let lastPingTimestamp = new Date();

  setInterval(() => {
    const now = new Date();
    if (now.getTime() - lastPingTimestamp.getTime() > config.pingCheckIntervalInMilliseconds) {
      console.log(`No ping events from Streaming API for ${config.pingCheckIntervalInMilliseconds/1000} seconds, closing ws connection...`);
      ws.close();
      process.exit(1);
    }
  }, config.pingCheckIntervalInMilliseconds);

  ws.on('message', async (data: Buffer) => {
    const rawData = data.toString();

    if (rawData === 'Ping') {
      lastPingTimestamp = new Date();
      return;
    }

    try {
      const csprCloudClient = new CSPRCloudAPIClient(config.csprCloudApiUrl, config.csprCloudAccessKey);
      const event = JSON.parse(rawData) as ContractEvent<any>;

      console.log('Received Event:', event.data.name, event.data.data);

      const eventName = event.data.name;
      const deployHash = event.extra.deploy_hash;
      const timestamp = formatDate(event.timestamp);

      if (eventName === 'AgentRegistered') {
        const payload = event.data.data as AgentRegisteredPayload;
        const agentRepo = AppDataSource.getRepository(AgentEntity);

        // Resolve public key to human-readable format if needed, otherwise use the address
        let publicKey = payload.agent;
        try {
          const account = await csprCloudClient.getAccount(payload.agent);
          publicKey = account.data.public_key || payload.agent;
        } catch (e) {
          console.log('Could not resolve account via CSPR.cloud, using raw address');
        }

        const agent = agentRepo.create({
          public_key: publicKey,
          name: payload.name,
          description: '',
          metadata_uri: '',
          active_jobs: 0,
          timestamp: new Date(timestamp)
        });
        await agentRepo.save(agent);
        console.log(`Agent registered: ${payload.name} (${publicKey})`);

      } else if (eventName === 'TaskCreated') {
        const payload = event.data.data as TaskCreatedPayload;
        const taskRepo = AppDataSource.getRepository(TaskEntity);

        let creatorKey = payload.creator;
        try {
          const account = await csprCloudClient.getAccount(payload.creator);
          creatorKey = account.data.public_key || payload.creator;
        } catch (e) {}

        const task = taskRepo.create({
          id: payload.task_id,
          creator_public_key: creatorKey,
          budget_motes: payload.budget,
          status: 'Open',
          transaction_hash: deployHash,
          timestamp: new Date(timestamp)
        });
        await taskRepo.save(task);
        console.log(`Task created: ${payload.task_id} with budget ${payload.budget} motes`);

      } else if (eventName === 'TaskAssigned') {
        const payload = event.data.data as TaskAssignedPayload;
        const taskRepo = AppDataSource.getRepository(TaskEntity);
        const agentRepo = AppDataSource.getRepository(AgentEntity);

        let agentKey = payload.agent;
        try {
          const account = await csprCloudClient.getAccount(payload.agent);
          agentKey = account.data.public_key || payload.agent;
        } catch (e) {}

        // Update task status and assigned agent
        await taskRepo.update(payload.task_id, {
          assigned_agent_public_key: agentKey,
          status: 'InProgress'
        });

        // Increment active jobs for agent
        const agent = await agentRepo.findOneBy({ public_key: agentKey });
        if (agent) {
          agent.active_jobs += 1;
          await agentRepo.save(agent);
        }

        console.log(`Task ${payload.task_id} assigned to agent ${agentKey}`);

      } else if (eventName === 'TaskSubmitted') {
        const payload = event.data.data as TaskSubmittedPayload;
        const taskRepo = AppDataSource.getRepository(TaskEntity);

        await taskRepo.update(payload.task_id, {
          result_hash: payload.result_hash
        });
        console.log(`Result submitted for task ${payload.task_id}: ${payload.result_hash}`);

      } else if (eventName === 'TaskCompleted') {
        const payload = event.data.data as TaskCompletedPayload;
        const taskRepo = AppDataSource.getRepository(TaskEntity);
        const agentRepo = AppDataSource.getRepository(AgentEntity);

        const task = await taskRepo.findOneBy({ id: payload.task_id });
        if (task) {
          task.status = 'Completed';
          await taskRepo.save(task);

          if (task.assigned_agent_public_key) {
            const agent = await agentRepo.findOneBy({ public_key: task.assigned_agent_public_key });
            if (agent && agent.active_jobs > 0) {
              agent.active_jobs -= 1;
              await agentRepo.save(agent);
            }
          }
        }
        console.log(`Task ${payload.task_id} marked as completed`);

      } else if (eventName === 'ScoreUpdated') {
        const payload = event.data.data as ScoreUpdatedPayload;
        const reputationRepo = AppDataSource.getRepository(ReputationEntity);

        let agentKey = payload.agent;
        try {
          const account = await csprCloudClient.getAccount(payload.agent);
          agentKey = account.data.public_key || payload.agent;
        } catch (e) {}

        const reputationId = `${agentKey}_${payload.skill}`;
        let reputation = await reputationRepo.findOneBy({ id: reputationId });

        if (reputation) {
          reputation.score = payload.new_score;
          reputation.timestamp = new Date(timestamp);
          await reputationRepo.save(reputation);
        } else {
          reputation = reputationRepo.create({
            id: reputationId,
            agent_public_key: agentKey,
            skill: payload.skill,
            score: payload.new_score,
            timestamp: new Date(timestamp)
          });
          await reputationRepo.save(reputation);
        }
        console.log(`Reputation updated for agent ${agentKey} in skill ${payload.skill}: ${payload.new_score}`);

      } else if (eventName === 'PriceUpdated') {
        const payload = event.data.data as PriceUpdatedPayload;
        const agentRepo = AppDataSource.getRepository(AgentEntity);

        let agentKey = payload.agent;
        try {
          const account = await csprCloudClient.getAccount(payload.agent);
          agentKey = account.data.public_key || payload.agent;
        } catch (e) {}

        await agentRepo.update(agentKey, {
          custom_price_motes: payload.custom_price
        });
        console.log(`On-chain custom price updated for agent ${agentKey}: ${payload.custom_price} motes`);

      } else if (eventName === 'RecommendedPriceUpdated') {
        const payload = event.data.data as RecommendedPriceUpdatedPayload;
        const agentRepo = AppDataSource.getRepository(AgentEntity);

        let agentKey = payload.agent;
        try {
          const account = await csprCloudClient.getAccount(payload.agent);
          agentKey = account.data.public_key || payload.agent;
        } catch (e) {}

        await agentRepo.update(agentKey, {
          recommended_price_motes: payload.recommended_price
        });
        console.log(`On-chain recommended price updated for agent ${agentKey}: ${payload.recommended_price} motes`);
      }

    } catch (err) {
      console.log('Error processing event:', err);
    }
  });

  ws.on('error', (err) => {
    console.log(`Received a WS error: ${err.message}`);
    ws.close();
    console.log('Disconnected from Streaming API');
    process.exit(1);
  });

  ws.on('close', () => {
    console.log('Disconnected from Streaming API');
    process.exit(1);
  });

  console.log('Event Handler started running...');
}

main();

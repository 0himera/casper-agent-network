import { config } from './config';
import WebSocket from 'ws';
import { pool } from "./db";
import { CSPRCloudAPIClient } from "./cspr-cloud/api-client";
import { formatDate } from "./utils";
import { RowDataPacket } from 'mysql2';
import { 
  ContractEvent, 
  AgentRegisteredPayload, 
  TaskCreatedPayload, 
  TaskAssignedPayload, 
  TaskSubmittedPayload, 
  TaskCompletedPayload, 
  ScoreUpdatedPayload,
  PriceUpdatedPayload,
  RecommendedPriceUpdatedPayload,
  TaskCancelledPayload,
  AgentUpdatedPayload,
  TaskDisputedPayload,
  PaymentClaimedPayload,
  MetadataUpdatedPayload,
  OwnershipTransferStartedPayload,
  OwnershipTransferredPayload,
  FeeDeductedPayload,
  FeeRateUpdatedPayload,
  AgentAvailabilityChangedPayload,
  TaskBudgetIncreasedPayload
} from "./events";

async function fetchWithRetry(url: string, options: RequestInit, retries = 3) {
  for (let i = 0; i < retries; i++) {
    try {
      const res = await fetch(url, options);
      if (!res.ok) {
        throw new Error(`HTTP error! status: ${res.status}`);
      }
      return res;
    } catch (err: any) {
      console.log(`Fetch failed (attempt ${i + 1}/${retries}):`, err.message);
      if (i === retries - 1) throw err;
      await new Promise(r => setTimeout(r, 1000 * (i + 1)));
    }
  }
}

let lastHealthCheckTime = 0;
let lastHealthCheckResult = false;
const HEALTH_CACHE_TTL_MS = 5000;

async function checkBackendHealth(): Promise<boolean> {
  const now = Date.now();
  if (now - lastHealthCheckTime < HEALTH_CACHE_TTL_MS) {
    return lastHealthCheckResult;
  }

  const rustBackendUrl = process.env.RUST_BACKEND_URL || 'http://localhost:3000';
  try {
    const res = await fetch(`${rustBackendUrl}/health`, { signal: AbortSignal.timeout(2000) });
    lastHealthCheckResult = res.ok;
  } catch (err: any) {
    console.log('Backend health check failed:', err.message || err);
    lastHealthCheckResult = false;
  }
  lastHealthCheckTime = now;
  return lastHealthCheckResult;
}

async function main() {
  console.log('Database initialized successfully via mysql2 pool.');

  const ws = new WebSocket(
      `${config.csprCloudStreamingUrl}/contract-events?contract_package_hash=${config.contractPackageHash}`,
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

  const internalServiceKey = process.env.INTERNAL_SERVICE_KEY || 'default_internal_key';
  const fetchHeaders = {
    'Content-Type': 'application/json',
    'Authorization': internalServiceKey
  };

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
        
        let publicKey = payload.agent;
        try {
          const account = await csprCloudClient.getAccount(payload.agent);
          publicKey = account.data.public_key || payload.agent;
        } catch (e) {
          console.log('Could not resolve account via CSPR.cloud, using raw address');
        }

        const [rows] = await pool.query<RowDataPacket[]>('SELECT * FROM agents WHERE public_key = ?', [publicKey]);
        const agent = rows[0];

        if (!agent) {
          await pool.execute(
            'INSERT INTO agents (public_key, name, description, metadata_uri, active_jobs, status, timestamp) VALUES (?, ?, ?, ?, ?, ?, ?)',
            [publicKey, payload.name, '', '', 0, 'active', new Date(timestamp)]
          );
          console.log(`Agent registered: ${payload.name} (${publicKey})`);
        } else {
          await pool.execute(
            'UPDATE agents SET name = ?, metadata_uri = IF(metadata_uri = "", "", metadata_uri), timestamp = ? WHERE public_key = ?',
            [payload.name, new Date(timestamp), publicKey]
          );
          console.log(`Agent already exists, updated metadata: ${payload.name} (${publicKey})`);
        }

      } else if (eventName === 'TaskCreated') {
        const payload = event.data.data as TaskCreatedPayload;
        
        let creatorKey = payload.creator;
        try {
          const account = await csprCloudClient.getAccount(payload.creator);
          creatorKey = account.data.public_key || payload.creator;
        } catch (e) {}

        const deadlineVal = payload.deadline ? payload.deadline.toString() : '0';

        const [rows] = await pool.query<RowDataPacket[]>('SELECT * FROM tasks WHERE id = ?', [payload.task_id]);
        const task = rows[0];

        if (!task) {
          await pool.execute(
            'INSERT INTO tasks (id, creator_public_key, budget_motes, status, transaction_hash, domain, prompt, deadline, timestamp) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)',
            [payload.task_id, creatorKey, payload.budget, 'Open', deployHash, 'defi_analysis', '', deadlineVal, new Date(timestamp)]
          );
          console.log(`Task created: ${payload.task_id} with budget ${payload.budget} motes, deadline ${deadlineVal}`);
        } else {
          await pool.execute(
            'UPDATE tasks SET transaction_hash = IFNULL(transaction_hash, ?), creator_public_key = IFNULL(creator_public_key, ?), deadline = IF(deadline = "0" OR deadline = 0, ?, deadline) WHERE id = ?',
            [deployHash, creatorKey, deadlineVal, payload.task_id]
          );
          console.log(`Task ${payload.task_id} already exists, updated transaction hash/creator/deadline`);
        }

      } else if (eventName === 'TaskAssigned') {
        const payload = event.data.data as TaskAssignedPayload;
        
        let agentKey = payload.agent;
        try {
          const account = await csprCloudClient.getAccount(payload.agent);
          agentKey = account.data.public_key || payload.agent;
        } catch (e) {}

        await pool.execute(
          'UPDATE tasks SET assigned_agent_public_key = ?, status = "InProgress" WHERE id = ?',
          [agentKey, payload.task_id]
        );

        await pool.execute(
          'UPDATE agents SET active_jobs = active_jobs + 1 WHERE public_key = ?',
          [agentKey]
        );

        console.log(`Task ${payload.task_id} assigned to agent ${agentKey}`);

        const [agentRows] = await pool.query<RowDataPacket[]>('SELECT * FROM agents WHERE public_key = ?', [agentKey]);
        const agent = agentRows[0];

        const rustBackendUrl = process.env.RUST_BACKEND_URL || 'http://localhost:3000';
        if (agent?.endpoint_url === 'autonomous') {
          console.log(`Agent ${agentKey} is autonomous, skipping backend execution for task ${payload.task_id}`);
        } else {
          const isHealthy = await checkBackendHealth();
          if (!isHealthy) {
            console.log(`Backend is unhealthy. Skipping automated execution for task ${payload.task_id}.`);
          } else {
            console.log(`Triggering automated execution for task ${payload.task_id} at ${rustBackendUrl}...`);
            fetchWithRetry(`${rustBackendUrl}/api/tasks/${payload.task_id}/execute`, {
              method: 'POST',
              headers: fetchHeaders
            }).catch(err => {
              console.log('Error triggering execution on backend:', err.message || err);
            });
          }
        }

      } else if (eventName === 'TaskSubmitted') {
        const payload = event.data.data as TaskSubmittedPayload;
        
        await pool.execute(
          'UPDATE tasks SET result_hash = ? WHERE id = ?',
          [payload.result_hash, payload.task_id]
        );
        console.log(`Result submitted for task ${payload.task_id}: ${payload.result_hash}`);

        const isHealthy = await checkBackendHealth();
        if (!isHealthy) {
          console.log(`Backend is unhealthy. Skipping automated validation for task ${payload.task_id}.`);
        } else {
          const rustBackendUrl = process.env.RUST_BACKEND_URL || 'http://localhost:3000';
          console.log(`Triggering validation for task ${payload.task_id}...`);
          fetchWithRetry(`${rustBackendUrl}/api/tasks/${payload.task_id}/validate`, {
            method: 'POST',
            headers: fetchHeaders
          }).catch(err => {
            console.log('Error triggering validation on backend:', err.message || err);
          });
        }

      } else if (eventName === 'TaskCompleted') {
        const payload = event.data.data as TaskCompletedPayload;
        
        const [taskRows] = await pool.query<RowDataPacket[]>('SELECT * FROM tasks WHERE id = ?', [payload.task_id]);
        const task = taskRows[0];

        if (task) {
          await pool.execute('UPDATE tasks SET status = "Completed" WHERE id = ?', [payload.task_id]);

          if (task.assigned_agent_public_key) {
            await pool.execute(
              'UPDATE agents SET active_jobs = GREATEST(0, active_jobs - 1) WHERE public_key = ?',
              [task.assigned_agent_public_key]
            );
          }
        }
        console.log(`Task ${payload.task_id} marked as completed`);

      } else if (eventName === 'ScoreUpdated') {
        const payload = event.data.data as ScoreUpdatedPayload;
        
        let agentKey = payload.agent;
        try {
          const account = await csprCloudClient.getAccount(payload.agent);
          agentKey = account.data.public_key || payload.agent;
        } catch (e) {}

        const reputationId = `${agentKey}_${payload.skill}`;
        const [repRows] = await pool.query<RowDataPacket[]>('SELECT * FROM reputations WHERE id = ?', [reputationId]);
        const reputation = repRows[0];

        if (reputation) {
          await pool.execute(
            'UPDATE reputations SET score = ?, timestamp = ? WHERE id = ?',
            [payload.new_score, new Date(timestamp), reputationId]
          );
        } else {
          await pool.execute(
            'INSERT INTO reputations (id, agent_public_key, skill, score, timestamp) VALUES (?, ?, ?, ?, ?)',
            [reputationId, agentKey, payload.skill, payload.new_score, new Date(timestamp)]
          );
        }
        console.log(`Reputation updated for agent ${agentKey} in skill ${payload.skill}: ${payload.new_score}`);

      } else if (eventName === 'PriceUpdated') {
        const payload = event.data.data as PriceUpdatedPayload;
        
        let agentKey = payload.agent;
        try {
          const account = await csprCloudClient.getAccount(payload.agent);
          agentKey = account.data.public_key || payload.agent;
        } catch (e) {}

        await pool.execute(
          'UPDATE agents SET custom_price_motes = ? WHERE public_key = ?',
          [payload.custom_price, agentKey]
        );
        console.log(`On-chain custom price updated for agent ${agentKey}: ${payload.custom_price} motes`);

      } else if (eventName === 'RecommendedPriceUpdated') {
        const payload = event.data.data as RecommendedPriceUpdatedPayload;
        
        let agentKey = payload.agent;
        try {
          const account = await csprCloudClient.getAccount(payload.agent);
          agentKey = account.data.public_key || payload.agent;
        } catch (e) {}

        await pool.execute(
          'UPDATE agents SET recommended_price_motes = ? WHERE public_key = ?',
          [payload.recommended_price, agentKey]
        );
        console.log(`On-chain recommended price updated for agent ${agentKey}: ${payload.recommended_price} motes`);

      } else if (eventName === 'TaskCancelled') {
        const payload = event.data.data as TaskCancelledPayload;
        
        const [taskRows] = await pool.query<RowDataPacket[]>('SELECT * FROM tasks WHERE id = ?', [payload.task_id]);
        const task = taskRows[0];

        if (task) {
          await pool.execute('UPDATE tasks SET status = "Cancelled" WHERE id = ?', [payload.task_id]);

          if (task.assigned_agent_public_key) {
            await pool.execute(
              'UPDATE agents SET active_jobs = GREATEST(0, active_jobs - 1) WHERE public_key = ?',
              [task.assigned_agent_public_key]
            );
          }
          console.log(`Task ${payload.task_id} marked as cancelled in DB`);
        }

      } else if (eventName === 'AgentUpdated') {
        const payload = event.data.data as AgentUpdatedPayload;
        
        let agentKey = payload.agent;
        try {
          const account = await csprCloudClient.getAccount(payload.agent);
          agentKey = account.data.public_key || payload.agent;
        } catch (e) {}

        await pool.execute(
          'UPDATE agents SET name = ?, timestamp = ? WHERE public_key = ?',
          [payload.name, new Date(timestamp), agentKey]
        );
        console.log(`Agent updated: ${payload.name} (${agentKey})`);

      } else if (eventName === 'TaskDisputed') {
        const payload = event.data.data as TaskDisputedPayload;
        
        await pool.execute(
          'UPDATE tasks SET status = "Disputed" WHERE id = ?',
          [payload.task_id]
        );
        console.log(`Task ${payload.task_id} marked as disputed by ${payload.disputer}`);

      } else if (eventName === 'PaymentClaimed') {
        const payload = event.data.data as PaymentClaimedPayload;
        
        await pool.execute(
          'UPDATE tasks SET status = "Completed" WHERE id = ?',
          [payload.task_id]
        );
        
        let agentKey = payload.agent;
        try {
          const account = await csprCloudClient.getAccount(payload.agent);
          agentKey = account.data.public_key || payload.agent;
        } catch (e) {}

        if (agentKey) {
          await pool.execute(
            'UPDATE agents SET active_jobs = GREATEST(0, active_jobs - 1) WHERE public_key = ?',
            [agentKey]
          );
        }
        console.log(`Payment claimed for task ${payload.task_id}: ${payload.amount} motes to agent ${agentKey}`);

      } else if (eventName === 'FeeDeducted') {
        const payload = event.data.data as FeeDeductedPayload;
        console.log(`Fee deducted for task ${payload.task_id}: fee=${payload.fee} payout=${payload.payout}`);

      } else if (eventName === 'FeeRateUpdated') {
        const payload = event.data.data as FeeRateUpdatedPayload;
        console.log(`Fee rate updated to ${payload.fee_bps} bps`);

      } else if (eventName === 'AgentAvailabilityChanged') {
        const payload = event.data.data as AgentAvailabilityChangedPayload;
        
        let agentKey = payload.agent;
        try {
          const account = await csprCloudClient.getAccount(payload.agent);
          agentKey = account.data.public_key || payload.agent;
        } catch (e) {}

        await pool.execute(
          'UPDATE agents SET is_available = ? WHERE public_key = ?',
          [payload.available ? 1 : 0, agentKey]
        );
        console.log(`Agent ${agentKey} availability changed to: ${payload.available}`);

      } else if (eventName === 'TaskBudgetIncreased') {
        const payload = event.data.data as TaskBudgetIncreasedPayload;
        
        await pool.execute(
          'UPDATE tasks SET budget_motes = ? WHERE id = ?',
          [payload.new_budget, payload.task_id]
        );
        console.log(`Task ${payload.task_id} budget increased to ${payload.new_budget} motes`);

      } else if (eventName === 'MetadataUpdated') {
        console.log(`Contract metadata updated`);

      } else if (eventName === 'OwnershipTransferStarted') {
        const payload = event.data.data as OwnershipTransferStartedPayload;
        console.log(`Ownership transfer started: ${payload.previous_owner} -> ${payload.new_owner}`);

      } else if (eventName === 'OwnershipTransferred') {
        const payload = event.data.data as OwnershipTransferredPayload;
        console.log(`Ownership transferred to: ${payload.new_owner}`);
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

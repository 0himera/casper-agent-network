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
  TaskBudgetIncreasedPayload,
  ValidatorRegisteredPayload,
  ValidatorStakedPayload,
  ValidatorUnstakedPayload,
  TreasuryDistributedPayload,
  TreasuryBurnedPayload
} from "./events";

async function fetchWithRetry(taskId: string, action: 'execute' | 'validate', options: RequestInit, retries = 3) {
  let safeUrl: string;
  try {
    if (!/^[a-zA-Z0-9_-]+$/.test(taskId)) {
      throw new Error(`SSRF Prevention: Invalid task ID format: ${taskId}`);
    }

    const allowedBackend = process.env.RUST_BACKEND_URL || 'http://localhost:3000';
    safeUrl = new URL(`/api/tasks/${taskId}/${action}`, allowedBackend).toString();
  } catch (err: any) {
    console.error("SSRF Validation failure:", err.message);
    throw err;
  }

  for (let i = 0; i < retries; i++) {
    try {
      const res = await fetch(safeUrl, options);
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

      // Security Check: Validate event payloads to prevent SSRF, Path Traversal, and Injection
      const payload = event.data.data;
      if (payload && typeof payload === 'object') {
        // Validate task IDs
        if ('task_id' in payload && payload.task_id !== undefined && payload.task_id !== null) {
          const taskId = String(payload.task_id);
          if (!/^[a-zA-Z0-9_-]+$/.test(taskId)) {
            console.error(`Security Warning: Rejected event due to invalid task_id format: "${taskId}"`);
            return;
          }
        }
        if ('parent_task_id' in payload && payload.parent_task_id !== undefined && payload.parent_task_id !== null) {
          const parentTaskId = String(payload.parent_task_id);
          if (!/^[a-zA-Z0-9_-]+$/.test(parentTaskId)) {
            console.error(`Security Warning: Rejected event due to invalid parent_task_id format: "${parentTaskId}"`);
            return;
          }
        }
        // Validate public key/account identifier fields
        const actorFields = ['agent', 'creator', 'validator', 'disputer'];
        for (const field of actorFields) {
          if (field in payload && payload[field] !== undefined && payload[field] !== null) {
            const actorVal = String(payload[field]);
            if (!/^(account-hash-)?[a-fA-F0-9]{64,66}$/.test(actorVal)) {
              console.error(`Security Warning: Rejected event due to invalid ${field} format: "${actorVal}"`);
              return;
            }
          }
        }
      }

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
            'INSERT INTO tasks (id, creator_public_key, budget_motes, status, transaction_hash, domain, prompt, deadline, timestamp, parent_task_id) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)',
            [payload.task_id, creatorKey, payload.budget, 'Open', deployHash, 'defi_analysis', '', deadlineVal, new Date(timestamp), payload.parent_task_id || null]
          );
          console.log(`Task created: ${payload.task_id} with budget ${payload.budget} motes, deadline ${deadlineVal}`);
        } else {
          await pool.execute(
            'UPDATE tasks SET transaction_hash = IFNULL(transaction_hash, ?), creator_public_key = IFNULL(creator_public_key, ?), deadline = IF(deadline = "0" OR deadline = 0, ?, deadline), parent_task_id = IFNULL(parent_task_id, ?) WHERE id = ?',
            [deployHash, creatorKey, deadlineVal, payload.parent_task_id || null, payload.task_id]
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
            fetchWithRetry(payload.task_id, 'execute', {
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
          fetchWithRetry(payload.task_id, 'validate', {
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

        // Phase 5.2: If smoothed leaderboard is active, the backend owns recommended price updates
        // based on smoothed_score. We ignore on-chain price updates to prevent clobbering the
        // off-chain price with legacy on-chain score-based prices after ordinary tasks.
        if (config.useSmoothedLeaderboard) {
          console.log(`Skipping on-chain recommended price update for agent ${agentKey} due to active smoothed leaderboard.`);
        } else {
          await pool.execute(
            'UPDATE agents SET recommended_price_motes = ? WHERE public_key = ?',
            [payload.recommended_price, agentKey]
          );
          console.log(`On-chain recommended price updated for agent ${agentKey}: ${payload.recommended_price} motes`);
        }

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
      } else if (eventName === 'ValidatorRegistered') {
        const payload = event.data.data as ValidatorRegisteredPayload;
        let validatorKey = payload.validator;
        try {
          const account = await csprCloudClient.getAccount(payload.validator);
          validatorKey = account.data.public_key || payload.validator;
        } catch (e) {}

        const [rows] = await pool.query<RowDataPacket[]>('SELECT * FROM validators WHERE public_key = ?', [validatorKey]);
        if (!rows[0]) {
          await pool.execute(
            'INSERT INTO validators (public_key, stake_motes, is_active, total_validations, timestamp) VALUES (?, 0, 1, 0, ?)',
            [validatorKey, new Date(timestamp)]
          );
        }
        console.log(`Validator registered: ${validatorKey}`);

      } else if (eventName === 'ValidatorStaked') {
        const payload = event.data.data as ValidatorStakedPayload;
        let validatorKey = payload.validator;
        try {
          const account = await csprCloudClient.getAccount(payload.validator);
          validatorKey = account.data.public_key || payload.validator;
        } catch (e) {}

        await pool.execute(
          'UPDATE validators SET stake_motes = stake_motes + ? WHERE public_key = ?',
          [payload.amount, validatorKey]
        );
        console.log(`Validator staked: ${validatorKey} amount: ${payload.amount}`);

      } else if (eventName === 'ValidatorUnstaked') {
        const payload = event.data.data as ValidatorUnstakedPayload;
        let validatorKey = payload.validator;
        try {
          const account = await csprCloudClient.getAccount(payload.validator);
          validatorKey = account.data.public_key || payload.validator;
        } catch (e) {}

        await pool.execute(
          'UPDATE validators SET stake_motes = GREATEST(0, stake_motes - ?) WHERE public_key = ?',
          [payload.amount, validatorKey]
        );
        console.log(`Validator unstaked: ${validatorKey} amount: ${payload.amount}`);

      } else if (eventName === 'TreasuryDistributed') {
        const payload = event.data.data as TreasuryDistributedPayload;
        console.log(`Treasury Distributed: yield=${payload.total_yield}, validators_paid=${payload.validators_paid}`);

      } else if (eventName === 'TreasuryBurned') {
        const payload = event.data.data as TreasuryBurnedPayload;
        console.log(`Treasury Burned: ${payload.burned_amount}`);

      } else if (eventName === 'ValidationSubmitted') {
        // ValidationSubmitted is an event, but the backend handles it immediately via the node. We can just log it here.
        console.log(`Validation Submitted by validator for a task`);
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

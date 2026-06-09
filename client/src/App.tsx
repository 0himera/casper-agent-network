import React, { useEffect, useState } from 'react';
import styled, { ThemeProvider } from 'styled-components';
import { useClickRef, ThemeModeType } from '@make-software/csprclick-ui';
import { AccountType, TransactionStatus } from '@make-software/csprclick-core-types';

import {
  AppTheme,
  formatAddress,
  buildRegisterAgentTx,
  buildCreateTaskTx,
  buildAssignTaskTx,
  buildSubmitResultTx,
  buildCompleteTaskTx,
  buildSetPriceTx,
  buildUpdateRecommendedPriceTx
} from '@/utils';
import { ClickTopBar, Container, HeroSection, PageFooter, Section } from '@/components';

const ContentSection = styled(Section)(({ theme }) =>
  theme.withMedia({
    maxWidth: ['100%', '95%', '1300px'],
    width: '100%',
    padding: '0 12px',
    margin: '30px auto'
  })
);

const DashboardWrapper = styled.div`
  display: flex;
  flex-direction: column;
  gap: 20px;
  background: rgba(255, 255, 255, 0.03);
  backdrop-filter: blur(10px);
  border: 1px solid rgba(255, 255, 255, 0.05);
  border-radius: 16px;
  padding: 24px;
`;

const TabsHeader = styled.div`
  display: flex;
  gap: 12px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  padding-bottom: 12px;
`;

const TabButton = styled.button<{ active: boolean }>`
  background: ${props => props.active ? 'linear-gradient(135deg, #FF6B6B 0%, #FF8E53 100%)' : 'transparent'};
  color: ${props => props.active ? '#ffffff' : 'rgba(255, 255, 255, 0.6)'};
  border: ${props => props.active ? 'none' : '1px solid rgba(255, 255, 255, 0.2)'};
  padding: 8px 16px;
  border-radius: 8px;
  cursor: pointer;
  font-weight: 600;
  transition: all 0.3s ease;

  &:hover {
    background: ${props => props.active ? 'linear-gradient(135deg, #FF6B6B 0%, #FF8E53 100%)' : 'rgba(255, 255, 255, 0.05)'};
    color: #ffffff;
  }
`;

const LayoutGrid = styled.div`
  display: grid;
  grid-template-columns: 2fr 1fr;
  gap: 24px;
  
  @media (max-width: 900px) {
    grid-template-columns: 1fr;
  }
`;

const CardList = styled.div`
  display: flex;
  flex-direction: column;
  gap: 16px;
`;

const Card = styled.div`
  background: rgba(255, 255, 255, 0.02);
  border: 1px solid rgba(255, 255, 255, 0.05);
  border-radius: 12px;
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: 8px;
`;

const CardTitle = styled.h3`
  margin: 0;
  font-size: 1.2rem;
  font-weight: 700;
  background: linear-gradient(135deg, #FF8E53 0%, #FF6B6B 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
`;

const FormSection = styled.div`
  background: rgba(255, 255, 255, 0.02);
  border: 1px solid rgba(255, 255, 255, 0.05);
  border-radius: 12px;
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  height: fit-content;
`;

const FormTitle = styled.h4`
  margin: 0;
  font-size: 1.1rem;
  color: #fff;
  border-bottom: 1px solid rgba(255,255,255,0.1);
  padding-bottom: 8px;
`;

const InputGroup = styled.div`
  display: flex;
  flex-direction: column;
  gap: 6px;
`;

const Label = styled.label`
  font-size: 0.8rem;
  color: rgba(255,255,255,0.6);
`;

const Input = styled.input`
  background: rgba(0,0,0,0.3);
  border: 1px solid rgba(255,255,255,0.1);
  color: #fff;
  padding: 8px 12px;
  border-radius: 6px;
  font-size: 0.9rem;
  
  &:focus {
    outline: none;
    border-color: #FF8E53;
  }
`;

const Select = styled.select`
  background: rgba(0,0,0,0.3);
  border: 1px solid rgba(255,255,255,0.1);
  color: #fff;
  padding: 8px 12px;
  border-radius: 6px;
  font-size: 0.9rem;
  
  option {
    background: #111;
  }
`;

const TextArea = styled.textarea`
  background: rgba(0,0,0,0.3);
  border: 1px solid rgba(255,255,255,0.1);
  color: #fff;
  padding: 8px 12px;
  border-radius: 6px;
  font-size: 0.9rem;
  min-height: 80px;
  resize: vertical;
`;

const Button = styled.button`
  background: linear-gradient(135deg, #FF6B6B 0%, #FF8E53 100%);
  color: #fff;
  border: none;
  padding: 10px 16px;
  border-radius: 6px;
  font-weight: 600;
  cursor: pointer;
  transition: opacity 0.2s;
  
  &:hover {
    opacity: 0.9;
  }
  
  &:disabled {
    background: #444;
    cursor: not-allowed;
  }
`;

const StatusBox = styled.div<{ error?: boolean }>`
  background: ${props => props.error ? 'rgba(231, 76, 60, 0.15)' : 'rgba(46, 204, 113, 0.15)'};
  color: ${props => props.error ? '#e74c3c' : '#2ecc71'};
  border: 1px solid ${props => props.error ? 'rgba(231, 76, 60, 0.3)' : 'rgba(46, 204, 113, 0.3)'};
  border-radius: 8px;
  padding: 12px;
  font-size: 0.85rem;
  margin-bottom: 16px;
  word-break: break-all;
`;

const App = () => {
  const clickRef = useClickRef();
  const [themeMode, setThemeMode] = useState<ThemeModeType>(ThemeModeType.dark);
  const [connectedAccount, setConnectedAccount] = useState<AccountType | null>(null);
  const [activeTab, setActiveTab] = useState<'agents' | 'tasks' | 'reputations'>('agents');
  
  const [agents, setAgents] = useState<any[]>([]);
  const [tasks, setTasks] = useState<any[]>([]);
  const [reputations, setReputations] = useState<any[]>([]);
  
  // Status feedback
  const [statusMsg, setStatusMsg] = useState<string>('');
  const [errorMsg, setErrorMsg] = useState<string>('');
  
  // Forms states
  const [agentForm, setAgentForm] = useState({
    name: 'Test Agent',
    description: 'DeFi analytics and review',
    metadataUri: 'https://ipfs.io/ipfs/QmMetadata',
    endpointUrl: 'hosted',
    apiKey: '',
    systemPrompt: 'You are a DeFi specialist.',
    skills: 'defi_analysis'
  });

  const [taskForm, setTaskForm] = useState({
    id: 'task_' + Math.floor(Math.random() * 100000),
    budget: '5',
    metadataUri: 'https://ipfs.io/ipfs/QmTaskMetadata',
    prompt: 'Review the arbitrage opportunity on CSPR.swap',
    domain: 'defi_analysis'
  });

  const [priceForm, setPriceForm] = useState({
    price: '3'
  });

  const [recPriceForm, setRecPriceForm] = useState({
    agent: '',
    price: '4'
  });

  const [assignForm, setAssignForm] = useState({
    taskId: '',
    agent: ''
  });

  const [submitForm, setSubmitForm] = useState({
    taskId: '',
    resultHash: 'ipfs://QmResultDataHash'
  });

  const [completeForm, setCompleteForm] = useState({
    taskId: '',
    skill: 'defi_analysis',
    score: 85
  });

  useEffect(() => {
    if (!clickRef) return;
    const handleSignedIn = (evt: any) => setConnectedAccount(evt.account);
    const handleSwitchedAccount = (evt: any) => setConnectedAccount(evt.account);
    const handleSignedOut = () => setConnectedAccount(null);

    clickRef.on('csprclick:signed_in', handleSignedIn);
    clickRef.on('csprclick:switched_account', handleSwitchedAccount);
    clickRef.on('csprclick:signed_out', handleSignedOut);

    const activeAcc = clickRef.getActiveAccount();
    if (activeAcc) setConnectedAccount(activeAcc);

    return () => {
      clickRef.off('csprclick:signed_in', handleSignedIn);
      clickRef.off('csprclick:switched_account', handleSwitchedAccount);
      clickRef.off('csprclick:signed_out', handleSignedOut);
    };
  }, [clickRef?.on]);

  const fetchData = async () => {
    try {
      const configApiUrl = (window as any).config?.agent_network_api_url || 'http://localhost:4000';
      
      const agentsRes = await fetch(`${configApiUrl}/agents`);
      if (agentsRes.ok) setAgents(await agentsRes.json());

      const tasksRes = await fetch(`${configApiUrl}/tasks`);
      if (tasksRes.ok) setTasks(await tasksRes.json());

      const repRes = await fetch(`${configApiUrl}/reputations`);
      if (repRes.ok) setReputations(await repRes.json());
    } catch (e) {
      console.log('Error fetching database data', e);
    }
  };

  useEffect(() => {
    fetchData();
    const interval = setInterval(fetchData, 5000);
    return () => clearInterval(interval);
  }, []);

  const handleTransactionSend = async (
    buildTxFn: (sender: string) => Promise<any>,
    successCallback?: (deployHash: string) => Promise<void>
  ) => {
    setStatusMsg('');
    setErrorMsg('');
    if (!connectedAccount) {
      setErrorMsg('Please connect your wallet first.');
      window.csprclick?.signIn();
      return;
    }

    const sender = connectedAccount.public_key;
    try {
      setStatusMsg('Building transaction...');
      const tx = await buildTxFn(sender);
      
      setStatusMsg('Sending to wallet for signature...');
      
      let calledSuccess = false;
      const onStatusUpdate = async (status: string, data: any) => {
        console.log("Tx Status Update:", status, data);
        const hash = data?.transactionHash || data?.deployHash;

        if (status === TransactionStatus.CANCELLED) {
          setStatusMsg('');
          setErrorMsg('Transaction cancelled by user.');
        } else if (status === TransactionStatus.ERROR) {
          setStatusMsg('');
          setErrorMsg(`Transaction error: ${data?.error} (${data?.errorData})`);
        } else if (status === TransactionStatus.SENT) {
          setStatusMsg(`Transaction sent! Hash: ${hash}. Waiting for block confirmation...`);
          if (successCallback && hash && !calledSuccess) {
            calledSuccess = true;
            await successCallback(hash);
          }
        } else if (status === TransactionStatus.PING) {
          setStatusMsg(`Transaction confirmed sent. Hash: ${hash}. Polling for block inclusion...`);
        } else if (status === TransactionStatus.PROCESSED) {
          if (data.csprCloudTransaction?.error_message === null) {
            setStatusMsg('Transaction processed successfully!');
            if (successCallback && hash && !calledSuccess) {
              calledSuccess = true;
              await successCallback(hash);
            }
            setTimeout(() => setStatusMsg(''), 4000);
            fetchData();
          } else {
            setStatusMsg('');
            setErrorMsg(`Execution failed: ${data.csprCloudTransaction?.error_message}`);
          }
        }
      };

      await clickRef?.send(tx, sender, onStatusUpdate);
    } catch (err: any) {
      setStatusMsg('');
      setErrorMsg(err.message || String(err));
    }
  };

  // 1. Register Agent
  const handleRegisterAgent = async (e: React.FormEvent) => {
    e.preventDefault();
    await handleTransactionSend(
      async (sender) => buildRegisterAgentTx(sender, agentForm.name, agentForm.description, agentForm.metadataUri),
      async (deployHash) => {
        // Also call backend register endpoint to start the benchmark
        const configApiUrl = (window as any).config?.agent_network_api_url || 'http://localhost:4000';
        const backendUrl = configApiUrl.replace(':4000', ':3000'); // Rust API runs on 3000
        
        await fetch(`${backendUrl}/api/agents/register`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            public_key: connectedAccount?.public_key,
            name: agentForm.name,
            description: agentForm.description,
            metadata_uri: agentForm.metadataUri,
            endpoint_url: agentForm.endpointUrl,
            api_key: agentForm.apiKey,
            system_prompt: agentForm.systemPrompt,
            skills: agentForm.skills.split(',').map(s => s.trim())
          })
        });
      }
    );
  };

  // 2. Create Task
  const handleCreateTask = async (e: React.FormEvent) => {
    e.preventDefault();
    const budgetMotes = (Number(taskForm.budget) * 1_000_000_000).toString();
    
    await handleTransactionSend(
      async (sender) => buildCreateTaskTx(sender, taskForm.id, budgetMotes, taskForm.metadataUri),
      async (deployHash) => {
        // Post details to backend API so it records the prompt & domain immediately
        const configApiUrl = (window as any).config?.agent_network_api_url || 'http://localhost:4000';
        const backendUrl = configApiUrl.replace(':4000', ':3000');
        
        await fetch(`${backendUrl}/api/tasks`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            id: taskForm.id,
            creator_public_key: connectedAccount?.public_key,
            budget_motes: Number(budgetMotes),
            transaction_hash: deployHash,
            domain: taskForm.domain,
            prompt: taskForm.prompt
          })
        });
        
        // Regenerate random task id
        setTaskForm(prev => ({ ...prev, id: 'task_' + Math.floor(Math.random() * 100000) }));
      }
    );
  };

  // 3. Set Price
  const handleSetPrice = async (e: React.FormEvent) => {
    e.preventDefault();
    const priceMotes = (Number(priceForm.price) * 1_000_000_000).toString();
    await handleTransactionSend(async (sender) => buildSetPriceTx(sender, priceMotes));
  };

  // 4. Set Recommended Price (Admin)
  const handleSetRecPrice = async (e: React.FormEvent) => {
    e.preventDefault();
    const priceMotes = (Number(recPriceForm.price) * 1_000_000_000).toString();
    await handleTransactionSend(async (sender) => buildUpdateRecommendedPriceTx(sender, recPriceForm.agent, priceMotes));
  };

  // 5. Assign Task
  const handleAssignTask = async (e: React.FormEvent) => {
    e.preventDefault();
    await handleTransactionSend(async (sender) => buildAssignTaskTx(sender, assignForm.taskId, assignForm.agent));
  };

  // 6. Submit Result
  const handleSubmitResult = async (e: React.FormEvent) => {
    e.preventDefault();
    await handleTransactionSend(async (sender) => buildSubmitResultTx(sender, submitForm.taskId, submitForm.resultHash));
  };

  // 7. Complete Task
  const handleCompleteTask = async (e: React.FormEvent) => {
    e.preventDefault();
    await handleTransactionSend(async (sender) => buildCompleteTaskTx(sender, completeForm.taskId, completeForm.skill, Number(completeForm.score)));
  };

  return (
    <ThemeProvider theme={AppTheme[themeMode]}>
      <ClickTopBar
        themeMode={themeMode}
        onThemeSwitch={() =>
          setThemeMode(themeMode === ThemeModeType.light ? ThemeModeType.dark : ThemeModeType.light)
        }
      />
      <Container>
        <HeroSection isConnected={!!connectedAccount} />
        
        <ContentSection id="dashboard-tabs">
          {statusMsg && <StatusBox>{statusMsg}</StatusBox>}
          {errorMsg && <StatusBox error>{errorMsg}</StatusBox>}

          <DashboardWrapper>
            <TabsHeader>
              <TabButton active={activeTab === 'agents'} onClick={() => setActiveTab('agents')}>
                🤖 Agents Registry
              </TabButton>
              <TabButton active={activeTab === 'tasks'} onClick={() => setActiveTab('tasks')}>
                💼 Job Board
              </TabButton>
              <TabButton active={activeTab === 'reputations'} onClick={() => setActiveTab('reputations')}>
                🏅 Reputation System
              </TabButton>
            </TabsHeader>

            {activeTab === 'agents' && (
              <LayoutGrid>
                <div>
                  <h3>Registered Agents</h3>
                  <CardList>
                    {agents.length === 0 ? (
                      <div>No registered agents found.</div>
                    ) : (
                      agents.map((agent, i) => (
                        <Card key={i}>
                          <CardTitle>{agent.name}</CardTitle>
                          <div style={{ fontSize: '0.8rem', fontFamily: 'monospace', color: '#aaa' }}>
                            PK: {agent.public_key}
                          </div>
                          <div>{agent.description}</div>
                          <div><strong>Status:</strong> {agent.status}</div>
                          <div><strong>Custom Price:</strong> {Number(agent.custom_price_motes) / 1_000_000_000} CSPR</div>
                          <div><strong>Recommended Price:</strong> {Number(agent.recommended_price_motes) / 1_000_000_000} CSPR</div>
                          <div><strong>Active Jobs:</strong> {agent.active_jobs}</div>
                        </Card>
                      ))
                    )}
                  </CardList>
                </div>
                
                <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
                  <FormSection>
                    <FormTitle>Register AI Agent</FormTitle>
                    <form onSubmit={handleRegisterAgent} style={{ display: 'flex', flexDirection: 'column', gap: '10px' }}>
                      <InputGroup>
                        <Label>Name</Label>
                        <Input value={agentForm.name} onChange={e => setAgentForm({...agentForm, name: e.target.value})} required />
                      </InputGroup>
                      <InputGroup>
                        <Label>Description</Label>
                        <Input value={agentForm.description} onChange={e => setAgentForm({...agentForm, description: e.target.value})} required />
                      </InputGroup>
                      <InputGroup>
                        <Label>Metadata URI</Label>
                        <Input value={agentForm.metadataUri} onChange={e => setAgentForm({...agentForm, metadataUri: e.target.value})} required />
                      </InputGroup>
                      <InputGroup>
                        <Label>Endpoint URL (or 'hosted')</Label>
                        <Input value={agentForm.endpointUrl} onChange={e => setAgentForm({...agentForm, endpointUrl: e.target.value})} required />
                      </InputGroup>
                      <InputGroup>
                        <Label>API Key (Optional)</Label>
                        <Input value={agentForm.apiKey} onChange={e => setAgentForm({...agentForm, apiKey: e.target.value})} />
                      </InputGroup>
                      <InputGroup>
                        <Label>System Prompt (Optional)</Label>
                        <TextArea value={agentForm.systemPrompt} onChange={e => setAgentForm({...agentForm, systemPrompt: e.target.value})} />
                      </InputGroup>
                      <InputGroup>
                        <Label>Skills (comma separated)</Label>
                        <Input value={agentForm.skills} onChange={e => setAgentForm({...agentForm, skills: e.target.value})} required />
                      </InputGroup>
                      <Button type="submit">Sign & Register Agent</Button>
                    </form>
                  </FormSection>

                  <FormSection>
                    <FormTitle>Set Custom Price (Agent)</FormTitle>
                    <form onSubmit={handleSetPrice} style={{ display: 'flex', flexDirection: 'column', gap: '10px' }}>
                      <InputGroup>
                        <Label>Price (CSPR)</Label>
                        <Input type="number" step="0.1" value={priceForm.price} onChange={e => setPriceForm({price: e.target.value})} required />
                      </InputGroup>
                      <Button type="submit">Set Custom Price</Button>
                    </form>
                  </FormSection>

                  <FormSection>
                    <FormTitle>Set Recommended Price (Admin)</FormTitle>
                    <form onSubmit={handleSetRecPrice} style={{ display: 'flex', flexDirection: 'column', gap: '10px' }}>
                      <InputGroup>
                        <Label>Agent Public Key</Label>
                        <Input value={recPriceForm.agent} onChange={e => setRecPriceForm({...recPriceForm, agent: e.target.value})} required />
                      </InputGroup>
                      <InputGroup>
                        <Label>Price (CSPR)</Label>
                        <Input type="number" step="0.1" value={recPriceForm.price} onChange={e => setRecPriceForm({...recPriceForm, price: e.target.value})} required />
                      </InputGroup>
                      <Button type="submit">Update Recommended Price</Button>
                    </form>
                  </FormSection>
                </div>
              </LayoutGrid>
            )}

            {activeTab === 'tasks' && (
              <LayoutGrid>
                <div>
                  <h3>Job Board</h3>
                  <CardList>
                    {tasks.length === 0 ? (
                      <div>No tasks found.</div>
                    ) : (
                      tasks.map((task, i) => (
                        <Card key={i}>
                          <CardTitle>ID: {task.id}</CardTitle>
                          <div><strong>Status:</strong> {task.status}</div>
                          <div><strong>Budget:</strong> {Number(task.budget_motes) / 1_000_000_000} CSPR</div>
                          <div><strong>Prompt:</strong> {task.prompt || 'No prompt in database'}</div>
                          <div><strong>Domain:</strong> {task.domain}</div>
                          <div style={{ fontSize: '0.8rem', color: '#aaa' }}>
                            <strong>Creator:</strong> {task.creator_public_key}
                          </div>
                          {task.assigned_agent_public_key && (
                            <div style={{ fontSize: '0.8rem', color: '#aaa' }}>
                              <strong>Assigned Agent:</strong> {task.assigned_agent_public_key}
                            </div>
                          )}
                          {task.result_hash && (
                            <div style={{ fontSize: '0.8rem', color: '#2ecc71', wordBreak: 'break-all' }}>
                              <strong>Result:</strong> {task.result_hash}
                            </div>
                          )}
                        </Card>
                      ))
                    )}
                  </CardList>
                </div>

                <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
                  <FormSection>
                    <FormTitle>Create Task (Escrow)</FormTitle>
                    <form onSubmit={handleCreateTask} style={{ display: 'flex', flexDirection: 'column', gap: '10px' }}>
                      <InputGroup>
                        <Label>Task ID (Unique)</Label>
                        <Input value={taskForm.id} onChange={e => setTaskForm({...taskForm, id: e.target.value})} required />
                      </InputGroup>
                      <InputGroup>
                        <Label>Budget (CSPR)</Label>
                        <Input type="number" step="0.1" value={taskForm.budget} onChange={e => setTaskForm({...taskForm, budget: e.target.value})} required />
                      </InputGroup>
                      <InputGroup>
                        <Label>Domain</Label>
                        <Select value={taskForm.domain} onChange={e => setTaskForm({...taskForm, domain: e.target.value})} required>
                          <option value="defi_analysis">DeFi Analysis</option>
                          <option value="code_review">Code Review</option>
                        </Select>
                      </InputGroup>
                      <InputGroup>
                        <Label>Prompt</Label>
                        <TextArea value={taskForm.prompt} onChange={e => setTaskForm({...taskForm, prompt: e.target.value})} required />
                      </InputGroup>
                      <InputGroup>
                        <Label>Metadata URI</Label>
                        <Input value={taskForm.metadataUri} onChange={e => setTaskForm({...taskForm, metadataUri: e.target.value})} required />
                      </InputGroup>
                      <Button type="submit">Post Task & Lock Escrow</Button>
                    </form>
                  </FormSection>

                  <FormSection>
                    <FormTitle>Assign Task</FormTitle>
                    <form onSubmit={handleAssignTask} style={{ display: 'flex', flexDirection: 'column', gap: '10px' }}>
                      <InputGroup>
                        <Label>Task ID</Label>
                        <Input value={assignForm.taskId} onChange={e => setAssignForm({...assignForm, taskId: e.target.value})} required />
                      </InputGroup>
                      <InputGroup>
                        <Label>Agent Public Key</Label>
                        <Input value={assignForm.agent} onChange={e => setAssignForm({...assignForm, agent: e.target.value})} required />
                      </InputGroup>
                      <Button type="submit">Assign Task</Button>
                    </form>
                  </FormSection>

                  <FormSection>
                    <FormTitle>Submit Result (Agent)</FormTitle>
                    <form onSubmit={handleSubmitResult} style={{ display: 'flex', flexDirection: 'column', gap: '10px' }}>
                      <InputGroup>
                        <Label>Task ID</Label>
                        <Input value={submitForm.taskId} onChange={e => setSubmitForm({...submitForm, taskId: e.target.value})} required />
                      </InputGroup>
                      <InputGroup>
                        <Label>Result text or IPFS CID</Label>
                        <TextArea value={submitForm.resultHash} onChange={e => setSubmitForm({...submitForm, resultHash: e.target.value})} required />
                      </InputGroup>
                      <Button type="submit">Submit Result</Button>
                    </form>
                  </FormSection>

                  <FormSection>
                    <FormTitle>Complete Task & Pay</FormTitle>
                    <form onSubmit={handleCompleteTask} style={{ display: 'flex', flexDirection: 'column', gap: '10px' }}>
                      <InputGroup>
                        <Label>Task ID</Label>
                        <Input value={completeForm.taskId} onChange={e => setCompleteForm({...completeForm, taskId: e.target.value})} required />
                      </InputGroup>
                      <InputGroup>
                        <Label>Skill category</Label>
                        <Input value={completeForm.skill} onChange={e => setCompleteForm({...completeForm, skill: e.target.value})} required />
                      </InputGroup>
                      <InputGroup>
                        <Label>Reputation score (0-100)</Label>
                        <Input type="number" value={completeForm.score} onChange={e => setCompleteForm({...completeForm, score: Number(e.target.value)})} required />
                      </InputGroup>
                      <Button type="submit">Complete & Pay Escrow</Button>
                    </form>
                  </FormSection>
                </div>
              </LayoutGrid>
            )}

            {activeTab === 'reputations' && (
              <div>
                <h3>Reputation Leaderboard</h3>
                <table style={{ width: '100%', borderCollapse: 'collapse', marginTop: '20px' }}>
                  <thead>
                    <tr style={{ borderBottom: '1px solid rgba(255,255,255,0.1)', textAlign: 'left' }}>
                      <th style={{ padding: '12px' }}>Agent Public Key</th>
                      <th style={{ padding: '12px' }}>Skill Domain</th>
                      <th style={{ padding: '12px' }}>On-Chain Reputation Score</th>
                      <th style={{ padding: '12px' }}>Last Updated</th>
                    </tr>
                  </thead>
                  <tbody>
                    {reputations.length === 0 ? (
                      <tr>
                        <td colSpan={4} style={{ padding: '12px', textAlign: 'center' }}>No reputation records found.</td>
                      </tr>
                    ) : (
                      reputations.map((rep, idx) => (
                        <tr key={idx} style={{ borderBottom: '1px solid rgba(255,255,255,0.05)' }}>
                          <td style={{ padding: '12px', fontFamily: 'monospace' }}>{rep.agent_public_key}</td>
                          <td style={{ padding: '12px' }}>{rep.skill}</td>
                          <td style={{ padding: '12px', fontWeight: 'bold', color: '#ff8e53' }}>{rep.score}</td>
                          <td style={{ padding: '12px' }}>{new Date(rep.timestamp).toLocaleString()}</td>
                        </tr>
                      ))
                    )}
                  </tbody>
                </table>
              </div>
            )}
          </DashboardWrapper>
        </ContentSection>
      </Container>
      <PageFooter />
    </ThemeProvider>
  );
};

export default App;

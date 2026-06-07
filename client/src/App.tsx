import React, { useEffect, useState } from 'react';
import styled, { ThemeProvider } from 'styled-components';
import { useClickRef, ThemeModeType } from '@make-software/csprclick-ui';
import { AccountType } from '@make-software/csprclick-core-types';

import { AppTheme, formatAddress } from '@/utils';
import { ClickTopBar, Container, HeroSection, PageFooter, Section } from '@/components';

const ContentSection = styled(Section)(({ theme }) =>
  theme.withMedia({
    maxWidth: ['100%', '800px', '1200px'],
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

const Grid = styled.div`
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
  gap: 20px;
  margin-top: 20px;
`;

const Card = styled.div`
  background: rgba(255, 255, 255, 0.02);
  border: 1px solid rgba(255, 255, 255, 0.05);
  border-radius: 12px;
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  transition: transform 0.2s ease, border-color 0.2s ease;

  &:hover {
    transform: translateY(-2px);
    border-color: rgba(255, 107, 107, 0.3);
  }
`;

const CardTitle = styled.h3`
  margin: 0;
  font-size: 1.2rem;
  font-weight: 700;
  background: linear-gradient(135deg, #FF8E53 0%, #FF6B6B 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
`;

const CardSubtitle = styled.div`
  font-size: 0.85rem;
  color: rgba(255, 255, 255, 0.5);
  font-family: monospace;
`;

const CardText = styled.p`
  margin: 0;
  font-size: 0.95rem;
  color: rgba(255, 255, 255, 0.8);
  line-height: 1.4;
`;

const StatBadge = styled.span<{ type?: string }>`
  background: ${props => {
    if (props.type === 'active') return 'rgba(46, 204, 113, 0.15)';
    if (props.type === 'completed') return 'rgba(52, 152, 219, 0.15)';
    return 'rgba(241, 196, 15, 0.15)';
  }};
  color: ${props => {
    if (props.type === 'active') return '#2ecc71';
    if (props.type === 'completed') return '#3498db';
    return '#f1c40f';
  }};
  padding: 4px 8px;
  border-radius: 6px;
  font-size: 0.8rem;
  font-weight: 600;
  width: fit-content;
`;

const SkillBadge = styled.span`
  background: rgba(255, 107, 107, 0.1);
  color: #FF6B6B;
  border: 1px solid rgba(255, 107, 107, 0.2);
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 0.75rem;
  font-weight: 500;
`;

const SkillsList = styled.div`
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 6px;
`;

// Fallback Mock Data
const mockAgents = [
  { public_key: '01a2b3...89c0', name: 'DeFi Arbitrageur #1', description: 'Monitors yield opportunities and DEX price anomalies on Casper network.', active_jobs: 1, skills: ['DeFi', 'Arbitrage', 'Trading'] },
  { public_key: '01c4d5...56f7', name: 'RWA Appraiser AI', description: 'Evaluates real estate property tokens and updates on-chain risk coefficients.', active_jobs: 0, skills: ['RWA', 'Valuation', 'Risk'] },
  { public_key: '01e8f9...12a3', name: 'Solidity Auditor Agent', description: 'Autonomous agent running static analysis and fuzz testing on smart contracts.', active_jobs: 2, skills: ['Security', 'Auditing', 'Solidity'] }
];

const mockTasks = [
  { id: 'task-01', creator_public_key: '01f3e5...78b2', budget_motes: '5000000000', status: 'InProgress', description: 'Analyze yield on CSPR.swap liquidity pool.', assigned_agent: 'DeFi Arbitrageur #1' },
  { id: 'task-02', creator_public_key: '01f3e5...78b2', budget_motes: '10000000000', status: 'Open', description: 'Audit new ERC-20 token contract.', assigned_agent: null },
  { id: 'task-03', creator_public_key: '01d4a6...98c1', budget_motes: '15000000000', status: 'Completed', description: 'Evaluate collateral scoring for real estate token pool.', assigned_agent: 'RWA Appraiser AI' }
];

const App = () => {
  const clickRef = useClickRef();
  const [themeMode, setThemeMode] = useState<ThemeModeType>(ThemeModeType.dark);
  const [connectedAccount, setConnectedAccount] = useState<AccountType | null>(null);
  const [activeTab, setActiveTab] = useState<'agents' | 'tasks'>('agents');
  const [agents, setAgents] = useState(mockAgents);
  const [tasks, setTasks] = useState(mockTasks);

  useEffect(() => {
    if (!clickRef) return;

    const handleSignedIn = (evt: any) => setConnectedAccount(evt.account);
    const handleSwitchedAccount = (evt: any) => setConnectedAccount(evt.account);
    const handleSignedOut = () => setConnectedAccount(null);

    clickRef.on('csprclick:signed_in', handleSignedIn);
    clickRef.on('csprclick:switched_account', handleSwitchedAccount);
    clickRef.on('csprclick:signed_out', handleSignedOut);

    // Initial check
    const activeAcc = clickRef.getActiveAccount();
    if (activeAcc) setConnectedAccount(activeAcc);

    return () => {
      clickRef.off('csprclick:signed_in', handleSignedIn);
      clickRef.off('csprclick:switched_account', handleSwitchedAccount);
      clickRef.off('csprclick:signed_out', handleSignedOut);
    };
  }, [clickRef?.on]);

  // Fetch real data from backend
  useEffect(() => {
    const fetchBackendData = async () => {
      try {
        const configApiUrl = (window as any).config?.agent_network_api_url || 'http://localhost:4000';
        
        const agentsRes = await fetch(`${configApiUrl}/agents`);
        if (agentsRes.ok) {
          const fetchedAgents = await agentsRes.json();
          if (fetchedAgents.length > 0) {
            setAgents(fetchedAgents);
          }
        }

        const tasksRes = await fetch(`${configApiUrl}/tasks`);
        if (tasksRes.ok) {
          const fetchedTasks = await tasksRes.json();
          if (fetchedTasks.length > 0) {
            setTasks(fetchedTasks);
          }
        }
      } catch (e) {
        console.log('Backend not running, displaying mock data', e);
      }
    };

    fetchBackendData();
  }, []);

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
          <DashboardWrapper>
            <TabsHeader>
              <TabButton active={activeTab === 'agents'} onClick={() => setActiveTab('agents')}>
                🤖 AI Agents Directory
              </TabButton>
              <TabButton active={activeTab === 'tasks'} onClick={() => setActiveTab('tasks')}>
                💼 Active Job Board
              </TabButton>
            </TabsHeader>

            {activeTab === 'agents' ? (
              <Grid>
                {agents.map((agent, index) => (
                  <Card key={index}>
                    <CardTitle>{agent.name}</CardTitle>
                    <CardSubtitle>Address: {formatAddress(agent.public_key)}</CardSubtitle>
                    <CardText>{agent.description || 'No description provided.'}</CardText>
                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginTop: 'auto' }}>
                      <StatBadge type={agent.active_jobs > 0 ? 'active' : ''}>
                        Active Jobs: {agent.active_jobs}
                      </StatBadge>
                      <SkillsList>
                        {((agent as any).skills || ['Agent']).map((skill: string, sIndex: number) => (
                          <SkillBadge key={sIndex}>{skill}</SkillBadge>
                        ))}
                      </SkillsList>
                    </div>
                  </Card>
                ))}
              </Grid>
            ) : (
              <Grid>
                {tasks.map((task, index) => (
                  <Card key={index}>
                    <CardTitle>{task.description}</CardTitle>
                    <CardSubtitle>Task ID: {task.id}</CardSubtitle>
                    <CardText>
                      <strong>Budget:</strong> {Number(task.budget_motes) / 1_000_000_000} CSPR
                    </CardText>
                    {task.assigned_agent && (
                      <CardText style={{ fontSize: '0.85rem', color: 'rgba(255,255,255,0.7)' }}>
                        👤 <strong>Assigned to:</strong> {task.assigned_agent}
                      </CardText>
                    )}
                    <StatBadge 
                      type={task.status === 'InProgress' ? 'active' : task.status === 'Completed' ? 'completed' : ''}
                      style={{ marginTop: 'auto' }}
                    >
                      Status: {task.status}
                    </StatBadge>
                  </Card>
                ))}
              </Grid>
            )}
          </DashboardWrapper>
        </ContentSection>
      </Container>
      <PageFooter />
    </ThemeProvider>
  );
};

export default App;

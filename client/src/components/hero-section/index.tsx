import React from 'react';
import { useClickRef } from '@make-software/csprclick-ui';
import {
  Container, Content,
  GreetingText,
  KillerAppText,
  SendTipButton,
} from './styled';

interface WelcomeProps {
  isConnected: boolean;
}

export const HeroSection: React.FC<WelcomeProps> = ({ isConnected }) => {
  const clickRef = useClickRef();

  const handleActionClick = () => {
    if (!isConnected) {
      window.csprclick.signIn();
    } else {
      // Scroll to registry or dashboard
      const element = document.getElementById('dashboard-tabs');
      if (element) {
        element.scrollIntoView({ behavior: 'smooth' });
      }
    }
  };

  return (
    <Container>
      <Content>
        <GreetingText>Casper Agent Network</GreetingText>
        <KillerAppText>
          Decentralized Proof-of-Skill protocol for autonomous AI agents. Compete, get evaluated, and build verifiable reputation.
        </KillerAppText>
        <SendTipButton onClick={handleActionClick}>
          {isConnected ? 'Explore Dashboard' : 'Connect Wallet'}
        </SendTipButton>
      </Content>
    </Container>
  );
};

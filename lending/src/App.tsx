import { Header } from "./components/Header";
import { TerminalHero } from "./components/TerminalHero";
import { EcosystemStack } from "./components/EcosystemStack";
import { HowItWorks } from "./components/HowItWorks";
import { NetworkOverview } from "./components/NetworkOverview";
import { ProtocolSpec } from "./components/ProtocolSpec";
import { ValidatorShowcase } from "./components/ValidatorShowcase";
import { YumaConsensus } from "./components/YumaConsensus";
import { AntiGaming } from "./components/AntiGaming";
import { PaymentPlayground } from "./components/PaymentPlayground";
import { McpIntegration } from "./components/McpIntegration";
import { Leaderboard } from "./components/Leaderboard";
import { UseCases } from "./components/UseCases";
import { DynamicPricing } from "./components/DynamicPricing";
import { RoadmapSection } from "./components/RoadmapSection";
import { FaqSection } from "./components/FaqSection";
import { NetworkMetrics } from "./components/NetworkMetrics";
import { Footer } from "./components/Footer";

export default function App() {
  return (
    <div className="flex flex-col min-h-screen bg-brand-bg text-brand-black font-sans antialiased overflow-x-hidden selection:bg-brand-black selection:text-brand-bg bg-grid-parallax">
      <Header />
      <main className="flex-1 pt-[57px]">
        <TerminalHero />
        <EcosystemStack />
        <HowItWorks />
        <NetworkOverview />
        <ProtocolSpec />
        <ValidatorShowcase />
        <YumaConsensus />
        <AntiGaming />
        <PaymentPlayground />
        <McpIntegration />
        <Leaderboard />
        <UseCases />
        <DynamicPricing />
        <RoadmapSection />
        <FaqSection />
        <NetworkMetrics />
      </main>
      <Footer />
    </div>
  );
}

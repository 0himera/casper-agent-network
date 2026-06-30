import { Header } from "./components/Header";
import { TerminalHero } from "./components/TerminalHero";
import { NetworkOverview } from "./components/NetworkOverview";
import { ProtocolSpec } from "./components/ProtocolSpec";
import { EscrowSandbox } from "./components/EscrowSandbox";
import { PaymentPlayground } from "./components/PaymentPlayground";
import { RoadmapSection } from "./components/RoadmapSection";
import { FaqSection } from "./components/FaqSection";
import { MetadataGenerator } from "./components/MetadataGenerator";
import { GasCalculator } from "./components/GasCalculator";
import { NetworkMetrics } from "./components/NetworkMetrics";
import { Footer } from "./components/Footer";

export default function App() {
  return (
    <div className="flex flex-col min-h-screen bg-brand-bg text-brand-black font-sans antialiased overflow-x-hidden selection:bg-brand-black selection:text-brand-bg bg-grid-parallax">
      <Header />
      <main className="flex-1">
        <TerminalHero />
        <NetworkOverview />
        <ProtocolSpec />
        <EscrowSandbox />
        <PaymentPlayground />
        <RoadmapSection />
        <FaqSection />
        <MetadataGenerator />
        <GasCalculator />
        <NetworkMetrics />
      </main>
      <Footer />
    </div>
  );
}

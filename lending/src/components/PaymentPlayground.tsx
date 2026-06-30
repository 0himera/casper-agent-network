import { useState } from "react";
import { PaymentSvg } from "./PaymentSvg";

const STATUS_MAP = {
  request: "HTTP_402: Payment required. Sending payment request...",
  paying: "X-CSPR-Payment: Sending micropayment headers...",
  received: "STATUS_200: Micropayment confirmed. Executing agent capability.",
  idle: "SYSTEM_READY: Simulate M2M x402 Micropayment workflow."
};

export function PaymentPlayground() {
  const [status, setStatus] = useState<keyof typeof STATUS_MAP>("idle");

  const startPayment = () => {
    if (status !== "idle" && status !== "received") return;
    setStatus("request");
    setTimeout(() => {
      setStatus("paying");
      setTimeout(() => setStatus("received"), 2500);
    }, 1500);
  };

  return (
    <section id="payments" className="grid grid-cols-1 lg:grid-cols-12 border-b border-brand-bg bg-brand-black text-brand-bg">
      <div className="hidden lg:flex lg:col-span-1 border-r border-brand-bg items-center justify-center bg-brand-black py-8">
        <span className="font-mono text-xs font-bold uppercase tracking-[0.2em] [writing-mode:vertical-lr] rotate-180">
          [ 03 / PAYMENT_SIM ]
        </span>
      </div>
      <div className="lg:col-span-11 px-6 py-20 md:px-12 lg:px-16">
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-12 items-center">
          <div className="lg:col-span-5 select-none">
            <span className="font-mono text-xs text-brand-orange">// MICROPAYMENT PROTOCOL</span>
            <h2 className="font-sans text-4xl md:text-5xl font-bold tracking-tighter uppercase mt-2 mb-6">
              x402 Micropayments
            </h2>
            <p className="font-sans text-base text-brand-bg/75 mb-8">
              Enable machine-to-machine streaming payments. Agents demand payments for API endpoints instantly, verified in real-time.
            </p>
            <div className="flex gap-4 font-mono text-xs">
              <button
                onClick={startPayment}
                disabled={status === "request" || status === "paying"}
                className="px-6 py-3 bg-brand-bg text-brand-black font-bold disabled:opacity-40 hover:bg-brand-black hover:text-brand-bg border border-brand-bg transition-colors"
              >
                REQUEST_RESOURCE
              </button>
              {status !== "idle" && (
                <button
                  onClick={() => setStatus("idle")}
                  className="px-6 py-3 border border-brand-bg text-brand-bg font-bold hover:bg-brand-bg hover:text-brand-black transition-colors"
                >
                  RESET_PAYMENT
                </button>
              )}
            </div>
          </div>

          <div className="lg:col-span-7 border border-brand-bg bg-brand-black p-6 flex flex-col justify-between min-h-[300px]">
            <div className="flex-1 flex items-center justify-center">
              <PaymentSvg status={status} />
            </div>
            <div className="mt-6 pt-4 border-t border-brand-bg/10 flex items-center justify-between font-mono text-xs">
              <span className="text-brand-orange font-bold animate-pulse">{STATUS_MAP[status]}</span>
              <span className="opacity-40">STAGE: {status.toUpperCase()}</span>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}

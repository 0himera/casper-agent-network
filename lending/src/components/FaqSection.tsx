import { FAQ_ITEMS } from "../utils/faqData";
import { FaqItem } from "./FaqItem";

export function FaqSection() {
  return (
    <section id="faq" className="grid grid-cols-1 lg:grid-cols-12 border-b border-brand-black bg-brand-bg text-brand-black">
      <div className="hidden lg:flex lg:col-span-1 border-r border-brand-black items-center justify-center bg-brand-bg py-8">
        <span className="font-mono text-xs font-bold uppercase tracking-[0.2em] [writing-mode:vertical-lr] rotate-180">
          [ 12 / FAQ ]
        </span>
      </div>
      <div className="lg:col-span-11 px-6 py-20 md:px-12 lg:px-16">
        <div className="max-w-4xl mx-auto">
          <div className="text-center mb-12 select-none">
            <span className="font-mono text-xs uppercase tracking-wider text-brand-orange">
              // PROTOCOL QUESTIONS
            </span>
            <h2 className="font-sans text-4xl md:text-5xl font-bold tracking-tighter uppercase mt-2 mb-4">
              Frequently Asked
            </h2>
            <p className="font-sans text-base text-brand-black/75 max-w-xl mx-auto">
              Find answers to standard inquiries regarding Proof-of-Skill smart escrows, payment headers, and validator consensus.
            </p>
          </div>

          <div className="space-y-4">
            {FAQ_ITEMS.map((item) => (
              <FaqItem key={item.id} item={item} />
            ))}
          </div>
        </div>
      </div>
    </section>
  );
}

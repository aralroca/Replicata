const STEPS = [
  {
    title: "Publish",
    body: "Describe a deterministic container run — image pinned by digest, command, seed — and escrow a USDC reward.",
  },
  {
    title: "Replicate",
    body: "Workers lock a bond, run it with no network access and commit a hash of the output before anyone reveals.",
  },
  {
    title: "Settle",
    body: "A strict majority of matching hashes gets paid; cheaters and no-shows lose their bond. All on-chain.",
  },
  {
    title: "Earn",
    body: "The verified result is sold over x402 to people and AI agents. Every sale is split with the replicators.",
  },
];

function Step({ index, title, body }: { index: number; title: string; body: string }) {
  return (
    <li className="card">
      <span className="mono text-sm text-accent">{String(index + 1).padStart(2, "0")}</span>
      <h3 className="mt-3 text-lg">{title}</h3>
      <p className="mt-2 text-sm leading-relaxed text-muted">{body}</p>
    </li>
  );
}

function Steps() {
  return (
    <ol className="mt-10 grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
      {STEPS.map((step, index) => (
        <Step key={step.title} index={index} {...step} />
      ))}
    </ol>
  );
}

export function HowItWorks() {
  return (
    <section aria-labelledby="how-title" className="py-12">
      <h2 id="how-title" className="text-center text-3xl sm:text-4xl">
        One bounty. <span className="text-accent">Independent proof.</span>
      </h2>
      <Steps />
    </section>
  );
}

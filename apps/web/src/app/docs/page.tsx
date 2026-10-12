import type { Metadata } from "next";

import { COORDINATOR_URL } from "@/lib/env";

import { CURL_EXAMPLES, PUBLISH_EXAMPLE, RUST_EXAMPLE, TS_EXAMPLE } from "./examples";

export const metadata: Metadata = { title: "Agents API" };

const SECTIONS = [
  { id: "curl", title: "Free endpoints (curl)", code: CURL_EXAMPLES },
  { id: "ts", title: "Pay for a verified result (TypeScript, x402)", code: TS_EXAMPLE },
  { id: "rust", title: "Rust (reqwest; x402-reqwest for paid routes)", code: RUST_EXAMPLE },
  { id: "publish", title: "Publish an experiment from an agent", code: PUBLISH_EXAMPLE },
];

function CodeSection({ id, title, code }: (typeof SECTIONS)[number]) {
  return (
    <section aria-labelledby={id} className="card grid gap-3">
      <h2 id={id} className="font-semibold">
        {title}
      </h2>
      <pre className="mono overflow-x-auto rounded-lg bg-background p-3 text-xs" tabIndex={0}>
        <code>{code.replaceAll("{{C}}", COORDINATOR_URL)}</code>
      </pre>
    </section>
  );
}

function Intro() {
  return (
    <header className="grid gap-2">
      <h1 className="text-2xl font-bold">Agents API</h1>
      <p className="text-muted">
        Everything in the UI is plain HTTP + x402 (USDC on Solana). No accounts, no API keys. Full machine-readable
        description: <a href={`${COORDINATOR_URL}/openapi.json`}>{COORDINATOR_URL}/openapi.json</a>.
      </p>
    </header>
  );
}

export default function DocsPage() {
  return (
    <div className="grid gap-6">
      <Intro />
      {SECTIONS.map((section) => (
        <CodeSection key={section.id} {...section} />
      ))}
    </div>
  );
}

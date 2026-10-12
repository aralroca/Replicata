import type { ReactNode } from "react";

import { COORDINATOR_URL, SITE_URL } from "@/lib/env";

export function replicateCommand(id: string, site = SITE_URL, coordinator = COORDINATOR_URL): string {
  return `curl -sSf ${site}/install.sh | sh && replicata run --coordinator ${coordinator} --once ${id}`;
}

export function badgeMarkdown(id: string, site = SITE_URL): string {
  return `[![Replicata](${site}/e/${id}/badge.svg)](${site}/e/${id})`;
}

const REPLICATE_HINT =
  "Run it on your machine (Docker required). You claim a slot, post a bond and get paid if your output matches the majority.";

export function Code({ code, label }: { code: string; label: string }) {
  return (
    <pre className="mono overflow-x-auto rounded-lg bg-background p-3 text-xs" tabIndex={0} aria-label={label}>
      <code>{code}</code>
    </pre>
  );
}

function Box({ id, title, children }: { id: string; title: string; children: ReactNode }) {
  return (
    <section aria-labelledby={id} className="card grid gap-2">
      <h2 id={id} className="font-semibold">
        {title}
      </h2>
      {children}
    </section>
  );
}

export function ReplicateBox({ id }: { id: string }) {
  return (
    <Box id="replicate-title" title="Replicate this">
      <p className="text-sm text-muted">{REPLICATE_HINT}</p>
      <Code code={replicateCommand(id)} label="Replicate command" />
    </Box>
  );
}

export function BadgeBox({ id }: { id: string }) {
  return (
    <Box id="badge-title" title="Badge">
      {/* eslint-disable-next-line @next/next/no-img-element -- dynamic SVG served by our own route */}
      <img src={`/e/${id}/badge.svg`} alt="Replication status badge" height={20} className="w-fit" />
      <Code code={badgeMarkdown(id)} label="Badge markdown" />
    </Box>
  );
}

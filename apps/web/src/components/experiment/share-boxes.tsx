import { COORDINATOR_URL, SITE_URL } from "@/lib/env";

export function replicateCommand(id: string, site = SITE_URL, coordinator = COORDINATOR_URL): string {
  return `curl -sSf ${site}/install.sh | sh && replicata run --coordinator ${coordinator} --once ${id}`;
}

export function badgeMarkdown(id: string, site = SITE_URL): string {
  return `[![Replicata](${site}/e/${id}/badge.svg)](${site}/e/${id})`;
}

function CodeBox({ id, title, hint, code }: { id: string; title: string; hint: string; code: string }) {
  return (
    <section aria-labelledby={id} className="card grid gap-2">
      <h2 id={id} className="font-semibold">
        {title}
      </h2>
      <p className="text-sm text-muted">{hint}</p>
      <pre className="mono overflow-x-auto rounded-lg bg-background p-3 text-xs" tabIndex={0}>
        <code>{code}</code>
      </pre>
    </section>
  );
}

export function ReplicateBox({ id }: { id: string }) {
  const hint = "Run it on your machine (Docker required). You claim a slot, post a bond and get paid if your output matches the majority.";

  return <CodeBox id="replicate-title" title="Replicate this" hint={hint} code={replicateCommand(id)} />;
}

export function BadgeBox({ id }: { id: string }) {
  return (
    <section aria-labelledby="badge-title" className="card grid gap-2">
      <h2 id="badge-title" className="font-semibold">
        Badge
      </h2>
      {/* eslint-disable-next-line @next/next/no-img-element -- dynamic SVG from our own route */}
      <img src={`/e/${id}/badge.svg`} alt="Replication status badge" height={20} />
      <pre className="mono overflow-x-auto rounded-lg bg-background p-3 text-xs" tabIndex={0}>
        <code>{badgeMarkdown(id)}</code>
      </pre>
    </section>
  );
}

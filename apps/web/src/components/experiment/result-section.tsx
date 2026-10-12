import { BuyResult } from "@/components/experiment/result-panel";
import { KeyLink } from "@/components/ui";
import { formatBps } from "@/lib/amounts";
import type { ExperimentView } from "@/lib/types";

type Props = { experiment: ExperimentView };

function ReplicatorList({ workers }: { workers: string[] }) {
  const items = workers.map((worker) => (
    <li key={worker}>
      <KeyLink prefix="/w" value={worker} />
    </li>
  ));

  return (
    <ul className="flex flex-wrap gap-2 text-sm" aria-label="Verified replicators">
      {items}
    </ul>
  );
}

function Replicators({ experiment }: Props) {
  const agreeing = experiment.replicators.slice(0, experiment.agree_count);

  return agreeing.length === 0 ? (
    <p className="text-sm text-muted">No verified replicators yet.</p>
  ) : (
    <ReplicatorList workers={agreeing} />
  );
}

function ResultHash({ hash }: { hash: string | null }) {
  return (
    <p className="text-sm">
      <span className="text-muted">Result hash: </span>
      <code className="mono break-all" data-testid="result-hash">
        {hash ?? "not sealed yet"}
      </code>
    </p>
  );
}

function Seal({ experiment }: Props) {
  const royalties = `Each paid access: ${formatBps(experiment.author_share_bps)} author, rest workers.`;

  return (
    <>
      <ResultHash hash={experiment.result_hash} />
      <Replicators experiment={experiment} />
      <p className="text-sm text-muted">{royalties}</p>
    </>
  );
}

export function ResultSection({ experiment }: Props) {
  return (
    <section aria-labelledby="result-title" className="card grid gap-4">
      <h2 id="result-title" className="font-semibold">
        Verified result
      </h2>
      <Seal experiment={experiment} />
      <BuyResult experiment={experiment} />
    </section>
  );
}

import Link from "next/link";

import { BuyResult } from "@/components/experiment/result-panel";
import { formatBps } from "@/lib/amounts";
import { shortHash } from "@/lib/hex";
import type { ExperimentView } from "@/lib/types";

function Replicators({ experiment }: { experiment: ExperimentView }) {
  const agreeing = experiment.replicators.slice(0, experiment.agree_count);

  return agreeing.length === 0 ? (
    <p className="text-sm text-muted">No verified replicators yet.</p>
  ) : (
    <ul className="flex flex-wrap gap-2 text-sm">
      {agreeing.map((worker) => (
        <li key={worker}>
          <Link href={`/w/${worker}`} className="mono">
            {shortHash(worker)}
          </Link>
        </li>
      ))}
    </ul>
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

export function ResultSection({ experiment }: { experiment: ExperimentView }) {
  const share = formatBps(experiment.author_share_bps);

  return (
    <section aria-labelledby="result-title" className="card grid gap-4">
      <h2 id="result-title" className="font-semibold">
        Verified result
      </h2>
      <ResultHash hash={experiment.result_hash} />
      <Replicators experiment={experiment} />
      <p className="text-sm text-muted">Each access pays royalties: {share} author, rest replicators.</p>
      <BuyResult experiment={experiment} />
    </section>
  );
}

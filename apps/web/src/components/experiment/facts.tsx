import type { ReactNode } from "react";

import { formatDeadline } from "@/components/experiment-card";
import { ExplorerLink, KeyLink, Stat } from "@/components/ui";
import { formatPrice, formatUsdc } from "@/lib/amounts";
import { shortHash } from "@/lib/hex";
import type { ClaimView, ExperimentView } from "@/lib/types";

type Props = { experiment: ExperimentView };

type Fact = { label: string; value: (e: ExperimentView) => ReactNode };

const ECONOMICS: Fact[] = [
  { label: "Reward", value: (e) => formatUsdc(e.reward) },
  { label: "Bond", value: (e) => formatUsdc(e.bond) },
  { label: "Replications (k / slots)", value: (e) => `${e.k} / ${e.max_claims}` },
  { label: "Access price", value: (e) => formatPrice(e.access_price) },
  { label: "Commit deadline", value: (e) => formatDeadline(e.commit_deadline) },
  { label: "Reveal deadline", value: (e) => formatDeadline(e.reveal_deadline) },
  { label: "Royalties paid", value: (e) => formatUsdc(e.royalties_paid) },
  { label: "Royalties pending", value: (e) => formatUsdc(e.royalties_pending) },
];

const CHAIN: Fact[] = [
  { label: "Experiment account", value: (e) => <ExplorerLink value={e.id} /> },
  { label: "Vault", value: (e) => <ExplorerLink value={e.vault} /> },
  { label: "Author", value: (e) => <KeyLink prefix="/r" value={e.authority} /> },
  { label: "Spec hash", value: (e) => <code className="mono">{shortHash(e.spec_hash)}</code> },
];

function FactList({ facts, experiment }: Props & { facts: Fact[] }) {
  return (
    <dl className="grid grid-cols-2 gap-4 text-sm sm:grid-cols-4">
      {facts.map((fact) => (
        <Stat key={fact.label} label={fact.label} value={fact.value(experiment)} />
      ))}
    </dl>
  );
}

export function Facts({ experiment }: Props) {
  return (
    <section aria-label="Experiment details" className="card grid gap-5">
      <FactList facts={ECONOMICS} experiment={experiment} />
      <FactList facts={CHAIN} experiment={experiment} />
    </section>
  );
}

function ClaimRow({ claim }: { claim: ClaimView }) {
  return (
    <tr className="border-t border-border">
      <td className="py-2">
        <KeyLink prefix="/w" value={claim.worker} />
      </td>
      <td>{claim.state}</td>
      <td className="mono">{claim.revealed_hash ? shortHash(claim.revealed_hash) : "—"}</td>
    </tr>
  );
}

const ClaimsHead = () => (
  <thead className="text-muted">
    <tr>
      <th scope="col">Worker</th>
      <th scope="col">State</th>
      <th scope="col">Revealed hash</th>
    </tr>
  </thead>
);

function ClaimsTable({ claims }: { claims: ClaimView[] }) {
  return (
    <table className="w-full text-left text-sm">
      <ClaimsHead />
      <tbody>
        {claims.map((claim) => (
          <ClaimRow key={claim.worker} claim={claim} />
        ))}
      </tbody>
    </table>
  );
}

export function Claims({ claims }: { claims: ClaimView[] }) {
  return claims.length === 0 ? null : (
    <section aria-labelledby="claims-title" className="card">
      <h2 id="claims-title" className="mb-3 font-semibold">
        Workers
      </h2>
      <ClaimsTable claims={claims} />
    </section>
  );
}

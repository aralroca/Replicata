import Link from "next/link";

import { formatDeadline } from "@/components/experiment-card";
import { ExplorerLink, Stat } from "@/components/ui";
import { formatPrice, formatUsdc } from "@/lib/amounts";
import { shortHash } from "@/lib/hex";
import type { ClaimView, ExperimentView } from "@/lib/types";

type Props = { experiment: ExperimentView };

function EconomicsFacts({ experiment: e }: Props) {
  return (
    <dl className="grid grid-cols-2 gap-4 text-sm sm:grid-cols-4">
      <Stat label="Reward" value={formatUsdc(e.reward)} />
      <Stat label="Bond" value={formatUsdc(e.bond)} />
      <Stat label="Replications (k / slots)" value={`${e.k} / ${e.max_claims}`} />
      <Stat label="Access price" value={formatPrice(e.access_price)} />
      <Stat label="Commit deadline" value={formatDeadline(e.commit_deadline)} />
      <Stat label="Reveal deadline" value={formatDeadline(e.reveal_deadline)} />
      <Stat label="Royalties paid" value={formatUsdc(e.royalties_paid)} />
      <Stat label="Royalties pending" value={formatUsdc(e.royalties_pending)} />
    </dl>
  );
}

function ChainFacts({ experiment: e }: Props) {
  return (
    <dl className="grid grid-cols-2 gap-4 text-sm sm:grid-cols-4">
      <Stat label="Experiment account" value={<ExplorerLink value={e.id} />} />
      <Stat label="Vault" value={<ExplorerLink value={e.vault} />} />
      <Stat label="Author" value={<Link href={`/r/${e.authority}`} className="mono">{shortHash(e.authority)}</Link>} />
      <Stat label="Spec hash" value={<code className="mono">{shortHash(e.spec_hash)}</code>} />
    </dl>
  );
}

export function Facts({ experiment }: Props) {
  return (
    <section aria-label="Experiment details" className="card grid gap-5">
      <EconomicsFacts experiment={experiment} />
      <ChainFacts experiment={experiment} />
    </section>
  );
}

function ClaimRow({ claim }: { claim: ClaimView }) {
  return (
    <tr className="border-t border-border">
      <td className="py-2">
        <Link href={`/w/${claim.worker}`} className="mono">{shortHash(claim.worker)}</Link>
      </td>
      <td>{claim.state}</td>
      <td className="mono">{claim.revealed_hash ? shortHash(claim.revealed_hash) : "—"}</td>
    </tr>
  );
}

export function Claims({ claims }: { claims: ClaimView[] }) {
  return claims.length === 0 ? null : (
    <section aria-labelledby="claims-title" className="card">
      <h2 id="claims-title" className="mb-3 font-semibold">Replicators</h2>
      <table className="w-full text-left text-sm">
        <thead className="text-muted">
          <tr><th scope="col">Worker</th><th scope="col">State</th><th scope="col">Revealed hash</th></tr>
        </thead>
        <tbody>{claims.map((claim) => <ClaimRow key={claim.worker} claim={claim} />)}</tbody>
      </table>
    </section>
  );
}

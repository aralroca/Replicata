"use client";

import Link from "next/link";

import { ExperimentList } from "@/components/experiment-feed";
import { ExplorerLink, LoadFailed, Loading, Stat } from "@/components/ui";
import { useAuthor, useWorker } from "@/hooks/use-coordinator";
import { formatUsdc } from "@/lib/amounts";
import { shortHash } from "@/lib/hex";
import type { AuthorView, WorkerView } from "@/lib/types";

function reputation(worker: WorkerView): string {
  const decided = worker.ok + worker.bad;

  return decided === 0 ? "—" : `${Math.round((worker.ok / decided) * 100)}%`;
}

function WorkerHistory({ history }: { history: WorkerView["history"] }) {
  return history.length === 0 ? (
    <p className="text-sm text-muted">No replications yet.</p>
  ) : (
    <ul className="grid gap-2 text-sm">
      {history.map((item) => (
        <li key={item.experiment}>
          <Link href={`/e/${item.experiment}`} className="mono">{shortHash(item.experiment)}</Link> · {item.state}
        </li>
      ))}
    </ul>
  );
}

function WorkerStats({ worker }: { worker: WorkerView }) {
  return (
    <dl className="card grid grid-cols-2 gap-4 sm:grid-cols-5">
      <Stat label="Claims" value={worker.claims} />
      <Stat label="Matched (ok)" value={worker.ok} />
      <Stat label="Disagreed (bad)" value={worker.bad} />
      <Stat label="Reputation" value={reputation(worker)} />
      <Stat label="Earned" value={formatUsdc(worker.earned)} />
    </dl>
  );
}

function ProfileTitle({ kind, pubkey }: { kind: string; pubkey: string }) {
  return (
    <header>
      <h1 className="text-2xl font-bold">{kind}</h1>
      <p className="mt-1 text-sm">
        <ExplorerLink value={pubkey} />
      </p>
    </header>
  );
}

export function WorkerProfile({ pubkey }: { pubkey: string }) {
  const { data, error } = useWorker(pubkey);

  if (error) return <LoadFailed what="the worker" error={error} />;

  return data ? (
    <div className="grid gap-6">
      <ProfileTitle kind="Worker" pubkey={pubkey} />
      <WorkerStats worker={data} />
      <section aria-label="History" className="card"><WorkerHistory history={data.history} /></section>
    </div>
  ) : <Loading />;
}

function AuthorStats({ author }: { author: AuthorView }) {
  return (
    <dl className="card grid grid-cols-2 gap-4 sm:grid-cols-3">
      <Stat label="Experiments" value={author.experiments.length} />
      <Stat label="Royalties paid" value={formatUsdc(author.royalties_paid)} />
      <Stat label="Royalties pending" value={formatUsdc(author.royalties_pending)} />
    </dl>
  );
}

export function ResearcherProfile({ pubkey }: { pubkey: string }) {
  const { data, error } = useAuthor(pubkey);

  if (error) return <LoadFailed what="the researcher" error={error} />;

  return data ? (
    <div className="grid gap-6">
      <ProfileTitle kind="Researcher" pubkey={pubkey} />
      <AuthorStats author={data} />
      <ExperimentList experiments={data.experiments} />
    </div>
  ) : <Loading />;
}

"use client";

import type { ReactNode } from "react";

import { ExperimentList } from "@/components/experiment-feed";
import { ExplorerLink, KeyLink, LoadFailed, Loading, Stat } from "@/components/ui";
import { useAuthor, useWorker } from "@/hooks/use-coordinator";
import { formatUsdc } from "@/lib/amounts";
import type { AuthorView, WorkerView } from "@/lib/types";

function reputation(worker: WorkerView): string {
  const decided = worker.ok + worker.bad;

  return decided === 0 ? "—" : `${Math.round((worker.ok / decided) * 100)}%`;
}

function HistoryList({ history }: { history: WorkerView["history"] }) {
  const items = history.map((item) => (
    <li key={item.experiment}>
      <KeyLink prefix="/e" value={item.experiment} /> · {item.state}
    </li>
  ));

  return <ul className="grid gap-2 text-sm">{items}</ul>;
}

function WorkerHistory({ history }: { history: WorkerView["history"] }) {
  return (
    <section aria-label="History" className="card">
      {history.length === 0 ? (
        <p className="text-sm text-muted">No replications yet.</p>
      ) : (
        <HistoryList history={history} />
      )}
    </section>
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

function ProfileHeader({ kind, pubkey }: { kind: string; pubkey: string }) {
  return (
    <header>
      <h1 className="text-2xl font-bold">{kind}</h1>
      <p className="mt-1 text-sm">
        <ExplorerLink value={pubkey} />
      </p>
    </header>
  );
}

function Profile({ kind, pubkey, children }: { kind: string; pubkey: string; children: ReactNode }) {
  return (
    <div className="grid gap-6">
      <ProfileHeader kind={kind} pubkey={pubkey} />
      {children}
    </div>
  );
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

export function WorkerProfile({ pubkey }: { pubkey: string }) {
  const { data, error } = useWorker(pubkey);

  if (error) return <LoadFailed what="the worker" error={error} />;
  if (!data) return <Loading />;

  return (
    <Profile kind="Worker" pubkey={pubkey}>
      <WorkerStats worker={data} />
      <WorkerHistory history={data.history} />
    </Profile>
  );
}

export function ResearcherProfile({ pubkey }: { pubkey: string }) {
  const { data, error } = useAuthor(pubkey);

  if (error) return <LoadFailed what="the researcher" error={error} />;
  if (!data) return <Loading />;

  return (
    <Profile kind="Researcher" pubkey={pubkey}>
      <AuthorStats author={data} />
      <ExperimentList experiments={data.experiments} />
    </Profile>
  );
}

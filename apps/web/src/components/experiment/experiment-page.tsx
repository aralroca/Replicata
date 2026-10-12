"use client";

import { experimentTitle, StatusPills } from "@/components/experiment-card";
import { Claims, Facts } from "@/components/experiment/facts";
import { ResultSection } from "@/components/experiment/result-section";
import { BadgeBox, ReplicateBox } from "@/components/experiment/share-boxes";
import { Timeline } from "@/components/experiment/timeline";
import { LoadFailed, Loading } from "@/components/ui";
import { useExperiment } from "@/hooks/use-coordinator";
import { ApiError } from "@/lib/api";
import type { ExperimentView } from "@/lib/types";

function Title({ experiment }: { experiment: ExperimentView }) {
  return (
    <header className="grid gap-2">
      <h1 className="text-2xl font-bold">{experimentTitle(experiment)}</h1>
      <StatusPills experiment={experiment} />
      {experiment.spec?.description && <p className="text-muted">{experiment.spec.description}</p>}
    </header>
  );
}

function Details({ experiment }: { experiment: ExperimentView }) {
  return (
    <>
      <Facts experiment={experiment} />
      <Claims claims={experiment.claims} />
      <ReplicateBox id={experiment.id} />
      <BadgeBox id={experiment.id} />
    </>
  );
}

function Experiment({ experiment }: { experiment: ExperimentView }) {
  return (
    <div className="grid gap-6">
      <Title experiment={experiment} />
      <Timeline experiment={experiment} />
      <ResultSection experiment={experiment} />
      <Details experiment={experiment} />
    </div>
  );
}

function NotIndexedYet() {
  return (
    <div className="card" role="status">
      <h1 className="text-xl font-semibold">Waiting for the indexer…</h1>
      <p className="mt-2 text-sm text-muted">If you just published it, it will show up in a few seconds.</p>
    </div>
  );
}

export function ExperimentPage({ id }: { id: string }) {
  const { data, error } = useExperiment(id);

  if (error instanceof ApiError && error.status === 404) return <NotIndexedYet />;
  if (error) return <LoadFailed what="the experiment" error={error} />;

  return data ? <Experiment experiment={data} /> : <Loading label="Loading experiment…" />;
}

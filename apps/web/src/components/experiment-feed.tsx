"use client";

import Link from "next/link";

import { ExperimentCard } from "@/components/experiment-card";
import { LoadFailed, Loading } from "@/components/ui";
import { useExperiments } from "@/hooks/use-coordinator";
import type { ExperimentView } from "@/lib/types";

function EmptyFeed() {
  return (
    <div className="card text-sm text-muted">
      No experiments yet. <Link href="/publish">Publish the first one</Link>.
    </div>
  );
}

export function ExperimentList({ experiments }: { experiments: ExperimentView[] }) {
  return experiments.length === 0 ? (
    <EmptyFeed />
  ) : (
    <ul className="grid gap-4">
      {experiments.map((experiment) => (
        <ExperimentCard key={experiment.id} experiment={experiment} />
      ))}
    </ul>
  );
}

export function ExperimentFeed() {
  const { data, error } = useExperiments();

  if (error) return <LoadFailed what="experiments" error={error} />;

  return data ? <ExperimentList experiments={data} /> : <Loading label="Loading experiments…" />;
}

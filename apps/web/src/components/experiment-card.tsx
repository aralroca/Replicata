import Link from "next/link";

import { Pill, Stat } from "@/components/ui";
import { formatUsdc } from "@/lib/amounts";
import { shortHash } from "@/lib/hex";
import { PHASE_META, STATUS_META } from "@/lib/status";
import type { ExperimentView } from "@/lib/types";

export function formatDeadline(unix: number): string {
  return new Date(unix * 1000).toISOString().replace("T", " ").slice(0, 16) + " UTC";
}

export function StatusPills({ experiment }: { experiment: ExperimentView }) {
  const status = STATUS_META[experiment.status];
  const phase = PHASE_META[experiment.phase];

  return (
    <div className="flex flex-wrap gap-2">
      <Pill tone={status.tone}>{status.label}</Pill>
      {experiment.status === "open" && <Pill tone={phase.tone}>{phase.label}</Pill>}
    </div>
  );
}

function CardStats({ experiment }: { experiment: ExperimentView }) {
  return (
    <dl className="mt-4 grid grid-cols-2 gap-3 text-sm sm:grid-cols-4">
      <Stat label="Reward" value={formatUsdc(experiment.reward)} />
      <Stat label="Replications" value={`${experiment.reveals_count}/${experiment.k}`} />
      <Stat label="Commit by" value={formatDeadline(experiment.commit_deadline)} />
      <Stat label="Reveal by" value={formatDeadline(experiment.reveal_deadline)} />
    </dl>
  );
}

export function ExperimentCard({ experiment }: { experiment: ExperimentView }) {
  const title = experiment.spec?.title ?? `Experiment ${shortHash(experiment.id)}`;

  return (
    <li className="card" data-testid="experiment-card">
      <div className="flex flex-wrap items-start justify-between gap-2">
        <h3 className="text-base font-semibold">
          <Link href={`/e/${experiment.id}`}>{title}</Link>
        </h3>
        <StatusPills experiment={experiment} />
      </div>
      <CardStats experiment={experiment} />
    </li>
  );
}

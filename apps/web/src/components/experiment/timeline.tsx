import { timeline, type StepState, type TimelineStep } from "@/lib/status";
import type { ExperimentView } from "@/lib/types";

const MARKS: Record<StepState, { icon: string; className: string; text: string }> = {
  done: { icon: "✓", className: "border-success text-success", text: "done" },
  active: { icon: "•", className: "border-accent text-accent", text: "in progress" },
  pending: { icon: "○", className: "border-border text-muted", text: "pending" },
  failed: { icon: "✕", className: "border-danger text-danger", text: "failed" },
};

function Step({ step }: { step: TimelineStep }) {
  const mark = MARKS[step.state];

  return (
    <li className="flex items-center gap-3" data-testid={`step-${step.key}`} data-state={step.state}>
      <span aria-hidden className={`grid size-8 place-items-center rounded-full border-2 ${mark.className}`}>
        {mark.icon}
      </span>
      <span>
        <span className="font-medium">{step.label}</span> <span className="text-muted">{step.detail}</span>
        <span className="sr-only"> ({mark.text})</span>
      </span>
    </li>
  );
}

export function Timeline({ experiment }: { experiment: ExperimentView }) {
  return (
    <section aria-labelledby="timeline-title" aria-live="polite" className="card">
      <h2 id="timeline-title" className="mb-4 font-semibold">
        Timeline
      </h2>
      <ol className="grid gap-3 sm:grid-cols-4">
        {timeline(experiment).map((step) => (
          <Step key={step.key} step={step} />
        ))}
      </ol>
    </section>
  );
}

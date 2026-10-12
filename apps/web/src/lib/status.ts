// Status / phase → UI labels, tones and the claim → commit → reveal → finalize timeline.
import type { ExperimentPhase, ExperimentStatus, ExperimentView } from "@/lib/types";

export type Tone = "neutral" | "info" | "success" | "warning" | "danger";

export const STATUS_META: Record<ExperimentStatus, { label: string; tone: Tone }> = {
  open: { label: "Open", tone: "info" },
  replicated: { label: "Replicated", tone: "success" },
  unresolved: { label: "Unresolved", tone: "warning" },
  cancelled: { label: "Cancelled", tone: "neutral" },
};

export const PHASE_META: Record<ExperimentPhase, { label: string; tone: Tone }> = {
  claiming: { label: "Claiming & committing", tone: "info" },
  revealing: { label: "Revealing", tone: "info" },
  finalizable: { label: "Ready to finalize", tone: "warning" },
  settled: { label: "Settled", tone: "neutral" },
};

export type StepState = "done" | "active" | "pending" | "failed";

export type TimelineStep = {
  key: "claims" | "commits" | "reveals" | "finalize";
  label: string;
  detail: string;
  state: StepState;
};

type Counts = Pick<
  ExperimentView,
  "k" | "max_claims" | "claims_count" | "commits_count" | "reveals_count" | "status" | "phase"
>;

function progressState(count: number, target: number, live: boolean): StepState {
  if (count >= target) {
    return "done";
  }

  return live ? "active" : "pending";
}

function finalizeStep(view: Counts): TimelineStep {
  const states: Record<ExperimentStatus, StepState> = {
    replicated: "done",
    unresolved: "failed",
    cancelled: "failed",
    open: view.phase === "finalizable" ? "active" : "pending",
  };
  const detail = view.status === "open" ? PHASE_META[view.phase].label : STATUS_META[view.status].label;

  return { key: "finalize", label: "Finalize", detail, state: states[view.status] };
}

export function timeline(view: Counts): TimelineStep[] {
  const open = view.status === "open";
  const claims = progressState(view.claims_count, view.k, open);
  const commits = progressState(view.commits_count, view.k, open && view.claims_count > 0);
  const reveals = progressState(view.reveals_count, view.k, open && view.phase === "revealing");

  return [
    { key: "claims", label: "Claims", detail: `${view.claims_count}/${view.k}`, state: claims },
    { key: "commits", label: "Commits", detail: `${view.commits_count}/${view.k}`, state: commits },
    { key: "reveals", label: "Reveals", detail: `${view.reveals_count}/${view.k}`, state: reveals },
    finalizeStep(view),
  ];
}

/** Short, human status used in badges and cards. */
export function statusSummary(view: Pick<ExperimentView, "status" | "agree_count" | "k" | "commits_count">) {
  const summaries: Record<ExperimentStatus, string> = {
    replicated: `Replicated ✅ ${view.agree_count}/${view.k}`,
    open: `Replicating ${view.commits_count}/${view.k}`,
    unresolved: "Unresolved",
    cancelled: "Cancelled",
  };

  return summaries[view.status];
}

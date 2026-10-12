import Link from "next/link";
import type { ReactNode } from "react";

import type { UiError } from "@/lib/errors";
import { explorerUrl } from "@/lib/explorer";
import { shortHash } from "@/lib/hex";
import type { Tone } from "@/lib/status";

const TONES: Record<Tone, string> = {
  neutral: "border-border text-muted",
  info: "border-accent/50 text-accent",
  success: "border-success/50 text-success",
  warning: "border-warning/50 text-warning",
  danger: "border-danger/50 text-danger",
};

export function Pill({ tone, children }: { tone: Tone; children: ReactNode }) {
  return (
    <span className={`inline-flex rounded-full border px-2 py-0.5 text-xs font-medium ${TONES[tone]}`}>
      {children}
    </span>
  );
}

export function Stat({ label, value }: { label: string; value: ReactNode }) {
  return (
    <div>
      <dt className="text-xs uppercase tracking-wide text-muted">{label}</dt>
      <dd className="mt-0.5 font-medium">{value}</dd>
    </div>
  );
}

export function ExplorerLink({ kind = "address", value }: { kind?: "address" | "tx"; value: string }) {
  return (
    <a href={explorerUrl(kind, value)} target="_blank" rel="noreferrer" className="mono">
      {shortHash(value)}
      <span className="sr-only"> (opens Solana Explorer)</span>
    </a>
  );
}

function ErrorAction({ action }: { action: NonNullable<UiError["action"]> }) {
  return action.href ? (
    <a href={action.href} target="_blank" rel="noreferrer" className="font-semibold">
      {action.label} →
    </a>
  ) : (
    <p className="font-semibold">{action.label}</p>
  );
}

export function ErrorAlert({ error }: { error: UiError }) {
  return (
    <div role="alert" data-kind={error.kind} className="rounded-lg border border-danger/60 bg-danger/10 p-4 text-sm">
      <p className="font-semibold text-danger">{error.title}</p>
      <p className="mt-1">{error.message}</p>
      {error.action && <div className="mt-2">{<ErrorAction action={error.action} />}</div>}
    </div>
  );
}

export function Loading({ label = "Loading…" }: { label?: string }) {
  return (
    <p role="status" className="text-sm text-muted">
      {label}
    </p>
  );
}

export function LoadFailed({ what, error }: { what: string; error: Error }) {
  return (
    <div role="alert" className="card text-sm">
      <p className="font-semibold text-danger">Could not load {what}</p>
      <p className="mt-1 text-muted">{error.message}</p>
      <Link href="/start">Is the coordinator running? See the setup guide.</Link>
    </div>
  );
}

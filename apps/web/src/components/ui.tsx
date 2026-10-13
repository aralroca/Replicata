import Link from "next/link";
import type { ReactNode } from "react";

import type { UiError } from "@/lib/errors";
import { explorerUrl } from "@/lib/explorer";
import { shortHash } from "@/lib/hex";
import type { Tone } from "@/lib/status";

/** In-app link to a pubkey page (`/w/…`, `/r/…`, `/e/…`) showing a shortened key. */
export function KeyLink({ prefix, value }: { prefix: "/w" | "/r" | "/e"; value: string }) {
  return (
    <Link href={`${prefix}/${value}`} className="mono">
      {shortHash(value)}
    </Link>
  );
}

const TONES: Record<Tone, string> = {
  neutral: "border-border bg-subtle text-muted",
  info: "border-accent/15 bg-accent/5 text-accent-strong",
  success: "border-success/15 bg-success/5 text-success",
  warning: "border-warning/15 bg-warning/5 text-warning",
  danger: "border-danger/15 bg-danger/5 text-danger",
};

export function Pill({ tone, children }: { tone: Tone; children: ReactNode }) {
  return (
    <span className={`inline-flex rounded-full border px-2.5 py-0.5 text-xs font-medium ${TONES[tone]}`}>
      {children}
    </span>
  );
}

export function Stat({ label, value }: { label: string; value: ReactNode }) {
  return (
    <div>
      <dt className="text-xs text-muted">{label}</dt>
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
    <div role="alert" data-kind={error.kind} className="rounded-2xl border border-danger/20 bg-danger/5 p-4 text-sm">
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
    <div role="alert" className="card flex flex-col items-center py-10 text-center text-sm">
      <p className="text-base font-medium">Could not load {what}</p>
      <p className="mt-1 text-muted">{error.message} — the coordinator may be starting up or unreachable.</p>
      <Link href="/start" className="btn btn-ghost mt-5">
        Setup guide <span aria-hidden>↗</span>
      </Link>
    </div>
  );
}

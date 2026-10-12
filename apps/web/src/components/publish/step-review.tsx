"use client";

import { useWalletUi, type UiWalletAccount } from "@wallet-ui/react";

import { ErrorAlert, Stat } from "@/components/ui";
import { usePublish } from "@/hooks/use-publish";
import { formatBps, formatPrice, formatUsdc } from "@/lib/amounts";
import { canonicalSpecJson } from "@/lib/spec/canonical";
import type { Economics } from "@/lib/tx/create-experiment";
import type { CoordinatorConfig, JobSpec } from "@/lib/types";

type Props = { spec: JobSpec; economics: Economics; config: CoordinatorConfig };

const STAGE_LABEL = { idle: "Publish & escrow reward", uploading: "Uploading spec…", signing: "Approve in your wallet…" };

function Summary({ spec, economics: e }: Omit<Props, "config">) {
  return (
    <dl className="grid grid-cols-2 gap-4 text-sm sm:grid-cols-4">
      <Stat label="Title" value={spec.title} />
      <Stat label="Reward (escrowed)" value={formatUsdc(e.reward)} />
      <Stat label="k / slots" value={`${e.k} / ${e.maxClaims}`} />
      <Stat label="Bond" value={formatUsdc(e.bond)} />
      <Stat label="Access price" value={formatPrice(e.accessPrice)} />
      <Stat label="Author share" value={formatBps(e.authorShareBps)} />
      <Stat label="Commit window" value={`${e.commitSecs / 60} min`} />
      <Stat label="Reveal window" value={`${e.revealSecs / 60} min`} />
    </dl>
  );
}

function PublishButton({ account, ...props }: Props & { account: UiWalletAccount }) {
  const { stage, error, publish } = usePublish(account, props.config);

  return (
    <div className="grid gap-3">
      <button type="button" className="btn btn-primary w-fit" disabled={stage !== "idle"} onClick={() => publish(props.spec, props.economics)}>
        {STAGE_LABEL[stage]}
      </button>
      {error && <ErrorAlert error={error} />}
    </div>
  );
}

export function StepReview(props: Props) {
  const { account } = useWalletUi();

  return (
    <section className="grid gap-4" aria-labelledby="review-title">
      <h2 id="review-title" className="text-lg font-semibold">3 · Review & publish</h2>
      <Summary spec={props.spec} economics={props.economics} />
      <pre className="mono max-h-48 overflow-auto rounded-lg bg-background p-3 text-xs" tabIndex={0} aria-label="Canonical spec JSON">{canonicalSpecJson(props.spec)}</pre>
      {account ? <PublishButton account={account} {...props} /> : <p role="status" className="text-sm">Connect a wallet (top right) to publish.</p>}
    </section>
  );
}

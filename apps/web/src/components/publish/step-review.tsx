"use client";

import { useWalletUi, type UiWalletAccount } from "@wallet-ui/react";

import { Code } from "@/components/experiment/share-boxes";
import { ErrorAlert, Stat } from "@/components/ui";
import { usePublish } from "@/hooks/use-publish";
import { formatBps, formatPrice, formatUsdc } from "@/lib/amounts";
import { canonicalSpecJson } from "@/lib/spec/canonical";
import type { Economics } from "@/lib/tx/create-experiment";
import type { CoordinatorConfig, JobSpec } from "@/lib/types";

type Props = { spec: JobSpec; economics: Economics; config: CoordinatorConfig };

const STAGE_LABEL = {
  idle: "Publish & escrow reward",
  uploading: "Uploading spec…",
  signing: "Approve in your wallet…",
  confirming: "Confirming on Solana…",
};

const SUMMARY: { label: string; value: (spec: JobSpec, e: Economics) => string }[] = [
  { label: "Title", value: (spec) => spec.title },
  { label: "Reward (escrowed)", value: (_, e) => formatUsdc(e.reward) },
  { label: "k / slots", value: (_, e) => `${e.k} / ${e.maxClaims}` },
  { label: "Bond", value: (_, e) => formatUsdc(e.bond) },
  { label: "Access price", value: (_, e) => formatPrice(e.accessPrice) },
  { label: "Author share", value: (_, e) => formatBps(e.authorShareBps) },
  { label: "Commit window", value: (_, e) => `${e.commitSecs / 60} min` },
  { label: "Reveal window", value: (_, e) => `${e.revealSecs / 60} min` },
];

function Summary({ spec, economics }: Omit<Props, "config">) {
  return (
    <dl className="grid grid-cols-2 gap-4 text-sm sm:grid-cols-4">
      {SUMMARY.map((item) => (
        <Stat key={item.label} label={item.label} value={item.value(spec, economics)} />
      ))}
    </dl>
  );
}

function PublishButton({ account, spec, economics, config }: Props & { account: UiWalletAccount }) {
  const { stage, error, publish } = usePublish(account, config);
  const onClick = () => publish(spec, economics);

  return (
    <div className="grid gap-3">
      <button type="button" className="btn btn-primary w-fit" disabled={stage !== "idle"} onClick={onClick}>
        {STAGE_LABEL[stage]}
      </button>
      {error && <ErrorAlert error={error} />}
    </div>
  );
}

function Publish(props: Props) {
  const { account } = useWalletUi();

  return account ? (
    <PublishButton account={account} {...props} />
  ) : (
    <p role="status" className="text-sm">
      Connect a wallet (top right) to publish.
    </p>
  );
}

export function StepReview(props: Props) {
  return (
    <section className="grid gap-4" aria-labelledby="review-title">
      <h2 id="review-title" className="text-lg font-semibold">
        3 · Review & publish
      </h2>
      <Summary spec={props.spec} economics={props.economics} />
      <Code code={canonicalSpecJson(props.spec)} label="Canonical spec JSON" />
      <Publish {...props} />
    </section>
  );
}

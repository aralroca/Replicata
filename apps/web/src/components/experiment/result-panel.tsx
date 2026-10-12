"use client";

import { useWalletAccountTransactionSigner, useWalletUi, type UiWalletAccount } from "@wallet-ui/react";
import Link from "next/link";

import { ErrorAlert } from "@/components/ui";
import { useBuyResult } from "@/hooks/use-buy-result";
import { formatPrice } from "@/lib/amounts";
import type { ExperimentView } from "@/lib/types";

type Props = { experiment: ExperimentView };

function buttonLabel(experiment: ExperimentView, pending: boolean): string {
  return pending ? "Paying…" : `Get verified result (${formatPrice(experiment.access_price)})`;
}

function PayButton({ experiment, account, chain }: Props & { account: UiWalletAccount; chain: `solana:${string}` }) {
  const signer = useWalletAccountTransactionSigner(account, chain);
  const { pending, error, downloaded, ready, buy } = useBuyResult(experiment.id, signer);

  return (
    <div className="grid gap-3">
      <button type="button" className="btn btn-primary w-fit" disabled={!ready || pending} onClick={buy}>
        {buttonLabel(experiment, pending)}
      </button>
      {error && <ErrorAlert error={error} />}
      {downloaded && <p role="status">Downloaded {downloaded}. Verify it with `replicata verify`.</p>}
    </div>
  );
}

function ConnectFirst({ experiment }: Props) {
  return (
    <div className="grid gap-2">
      <button type="button" className="btn btn-primary w-fit" disabled>
        {buttonLabel(experiment, false)}
      </button>
      <p className="text-sm text-muted">
        Connect a wallet (top right) to pay with USDC. New here? <Link href="/start">Get set up in 60 s</Link>.
      </p>
    </div>
  );
}

export function BuyResult({ experiment }: Props) {
  const { account, cluster } = useWalletUi();

  return account ? (
    <PayButton experiment={experiment} account={account} chain={cluster.id} />
  ) : (
    <ConnectFirst experiment={experiment} />
  );
}

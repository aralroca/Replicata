"use client";

import { useWalletAccountTransactionSigner, useWalletUi, type UiWalletAccount } from "@wallet-ui/react";
import Link from "next/link";

import { ErrorAlert } from "@/components/ui";
import { useBuyResult } from "@/hooks/use-buy-result";
import { formatPrice } from "@/lib/amounts";
import type { ExperimentView } from "@/lib/types";

type Props = { experiment: ExperimentView };

type PayProps = Props & { account: UiWalletAccount; chain: `solana:${string}` };

type BuyButtonProps = Props & { pending?: boolean; disabled: boolean; onClick?: () => void };

function BuyButton({ experiment, pending, disabled, onClick }: BuyButtonProps) {
  const label = pending ? "Paying…" : `Get verified result (${formatPrice(experiment.access_price)})`;

  return (
    <button type="button" className="btn btn-accent w-fit" disabled={disabled} onClick={onClick}>
      {label}
    </button>
  );
}

function PayButton({ experiment, account, chain }: PayProps) {
  const signer = useWalletAccountTransactionSigner(account, chain);
  const { pending, error, downloaded, ready, buy } = useBuyResult(experiment, signer);

  return (
    <div className="grid gap-3">
      <BuyButton experiment={experiment} pending={pending} disabled={!ready || pending} onClick={buy} />
      {error && <ErrorAlert error={error} />}
      {downloaded && <p role="status">Downloaded {downloaded}. Check it with `replicata verify`.</p>}
    </div>
  );
}

function ConnectFirst({ experiment }: Props) {
  return (
    <div className="grid gap-2">
      <BuyButton experiment={experiment} disabled />
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

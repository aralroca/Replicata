"use client";

import type { TransactionSigner } from "@solana/kit";
import { useState } from "react";

import { useConfig } from "@/hooks/use-coordinator";
import { errorText, type UiError } from "@/lib/errors";
import type { CoordinatorConfig } from "@/lib/types";
import { buyResult, saveFile, type PaymentOutcome } from "@/lib/x402/pay";

type BuyState = { pending: boolean; error: UiError | null; downloaded: string | null };

const IDLE: BuyState = { pending: false, error: null, downloaded: null };

function settle(outcome: PaymentOutcome): BuyState {
  if (outcome.error) {
    return { pending: false, error: outcome.error, downloaded: null };
  }
  saveFile(outcome.file);

  return { pending: false, error: null, downloaded: outcome.file.filename };
}

function runBuy(id: string, signer: TransactionSigner, config: CoordinatorConfig): Promise<BuyState> {
  const setup = { signer, network: config.network, rpcUrl: config.rpc_url, faucets: config.faucets };
  const failed = (error: unknown): BuyState => ({
    ...IDLE,
    error: { kind: "unknown", title: "Download failed", message: errorText(error) },
  });

  return buyResult(id, setup).then(settle).catch(failed);
}

/** Pays the x402 result endpoint from the wallet and downloads the verified bundle. */
export function useBuyResult(id: string, signer: TransactionSigner) {
  const { data: config } = useConfig();
  const [state, setState] = useState<BuyState>(IDLE);
  const buy = (current: CoordinatorConfig) => {
    setState({ ...IDLE, pending: true });
    runBuy(id, signer, current).then(setState);
  };

  return { ...state, ready: Boolean(config), buy: () => config && buy(config) };
}

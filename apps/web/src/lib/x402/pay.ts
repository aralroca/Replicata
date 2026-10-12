// Buy the verified result bundle over x402 and hand it to the browser as a download.
import type { TransactionSigner } from "@solana/kit";
import { wrapFetchWithPayment } from "@x402/fetch";

import { COORDINATOR_URL } from "@/lib/env";
import type { Faucets, UiError } from "@/lib/errors";
import { paymentError, thrownPaymentError, type PaidResponse } from "@/lib/x402/errors";
import { createPaymentClient } from "@/lib/x402/scheme";

export type ResultFile = { blob: Blob; filename: string; resultHash: string | null };

export type PaymentSetup = { signer: TransactionSigner; network: string; rpcUrl: string; faucets: Faucets };

export type PaymentOutcome = { file: ResultFile; error?: never } | { file?: never; error: UiError };

async function readPaidResponse(response: Response): Promise<PaidResponse> {
  const isJson = (response.headers.get("content-type") ?? "").includes("json");
  const body = isJson ? await response.json().catch(() => null) : null;

  return { status: response.status, body, retryAfter: response.headers.get("retry-after") };
}

export function filenameFrom(disposition: string | null, fallback: string): string {
  return /filename="?([^";]+)"?/.exec(disposition ?? "")?.[1] ?? fallback;
}

async function toOutcome(response: Response, id: string, faucets: Faucets): Promise<PaymentOutcome> {
  const error = response.ok ? null : paymentError(await readPaidResponse(response), faucets);
  const filename = filenameFrom(response.headers.get("content-disposition"), `${id}.tar.gz`);

  if (error) {
    return { error };
  }

  return { file: { blob: await response.blob(), filename, resultHash: response.headers.get("x-result-hash") } };
}

export async function buyResult(id: string, setup: PaymentSetup): Promise<PaymentOutcome> {
  const client = createPaymentClient(setup.signer, setup.network, setup.rpcUrl);
  const paidFetch = wrapFetchWithPayment(fetch, client);

  return paidFetch(`${COORDINATOR_URL}/v1/experiments/${id}/result`)
    .then((response) => toOutcome(response, id, setup.faucets))
    .catch((error: unknown) => ({ error: thrownPaymentError(error, setup.faucets) }));
}

export function saveFile({ blob, filename }: ResultFile): void {
  const url = URL.createObjectURL(blob);
  const link = Object.assign(document.createElement("a"), { href: url, download: filename });

  link.click();
  setTimeout(() => URL.revokeObjectURL(url), 1_000);
}

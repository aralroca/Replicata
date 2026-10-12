// HTTP outcomes of the paid result endpoint (docs/API.md "Payment failures") → UI errors.
import { errorText, insufficientFunds, txError, type Faucets, type UiError } from "@/lib/errors";

export type PaidResponse = {
  status: number;
  body: { error?: string; faucet?: string } | null;
  retryAfter: string | null;
};

const retryLater = (retryAfter: string | null): UiError => ({
  kind: "facilitator_unavailable",
  title: "Payment service unavailable",
  message: `The payment facilitator is not responding. You were not charged.`,
  action: { label: `Retry in ${retryAfter ?? "a few"} seconds` },
});

const NOT_REPLICATED: UiError = {
  kind: "not_replicated",
  title: "Not replicated yet",
  message: "This experiment has no verified result yet, so nothing can be bought. You were not charged.",
  action: { label: "Follow the timeline and come back once it is Replicated" },
};

const NOT_FOUND: UiError = {
  kind: "not_found",
  title: "Experiment not found",
  message: "The coordinator does not know this experiment.",
};

const SETTLEMENT_FAILED: UiError = {
  kind: "settlement_failed",
  title: "Payment could not be settled",
  message: "The payment was not settled on-chain and the result was not delivered.",
  action: { label: "Try again" },
};

const REPLAYED: UiError = {
  kind: "payment_replayed",
  title: "Payment already used",
  message: "That signed payment was already used once.",
  action: { label: "Try again to sign a fresh payment" },
};

function paymentRequired(response: PaidResponse, faucets: Faucets): UiError {
  const errors: Record<string, UiError> = {
    insufficient_funds: insufficientFunds(response.body?.faucet ?? faucets.usdc, "USDC"),
    settlement_failed: SETTLEMENT_FAILED,
  };

  return errors[response.body?.error ?? ""] ?? SETTLEMENT_FAILED;
}

function unexpected(response: PaidResponse): UiError {
  const detail = response.body?.error ? `: ${response.body.error}` : "";

  return {
    kind: "unknown",
    title: "Unexpected response",
    message: `The coordinator answered ${response.status}${detail}.`,
  };
}

/** `null` when the response delivered the resource. */
export function paymentError(response: PaidResponse, faucets: Faucets): UiError | null {
  const byStatus: Record<number, () => UiError> = {
    402: () => paymentRequired(response, faucets),
    404: () => NOT_FOUND,
    409: () => (response.body?.error === "payment_replayed" ? REPLAYED : NOT_REPLICATED),
    503: () => retryLater(response.retryAfter),
  };

  return response.status < 300 ? null : (byStatus[response.status]?.() ?? unexpected(response));
}

const NETWORK_DOWN = /failed to fetch|networkerror|load failed/i;
const OVER_BUDGET = /spendControls|exceeds|maxAmountPerPayment/i;

const NETWORK_ERROR = (text: string): UiError => ({
  kind: "network",
  title: "Coordinator unreachable",
  message: text,
  action: { label: "Try again" },
});

const OVER_BUDGET_ERROR: UiError = {
  kind: "unknown",
  title: "Payment request refused",
  message: "The coordinator asked for a different token or a higher price than shown. Nothing was paid.",
  action: { label: "Reload the page and check the price" },
};

/** Errors thrown by `@x402/fetch` (wallet rejection, unreachable coordinator, unexpected price…). */
export function thrownPaymentError(error: unknown, faucets: Faucets): UiError {
  const text = errorText(error);

  if (NETWORK_DOWN.test(text)) return NETWORK_ERROR(text);
  if (OVER_BUDGET.test(text)) return OVER_BUDGET_ERROR;

  return txError(error, faucets);
}

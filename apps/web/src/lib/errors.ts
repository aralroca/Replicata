// Turns wallet / transaction failures into something a user can act on.
import { getReplicataErrorMessage, type ReplicataError } from "@/generated";

export type UiErrorKind =
  | "insufficient_funds"
  | "user_rejected"
  | "facilitator_unavailable"
  | "not_replicated"
  | "settlement_failed"
  | "payment_replayed"
  | "not_found"
  | "program_error"
  | "network"
  | "unknown";

export type UiAction = { label: string; href?: string };

export type UiError = { kind: UiErrorKind; title: string; message: string; action?: UiAction };

export type Faucets = { sol: string; usdc: string };

const REJECTED =
  /user rejected|rejected the request|request rejected|denied by (the )?user|declined|cancell?ed by (the )?user|\b4001\b/i;
const NO_SOL = /insufficient (funds|lamports|balance)|attempt to debit an account but found no record/i;
const NO_TOKENS = /custom program error: 0x1\b/i;
const PROGRAM_HEX = /custom program error: 0x(17[0-9a-f]{2})/i;
const PROGRAM_DEC = /custom (?:program )?error(?: code)?:? #?(60\d\d)\b/i;

function programCode(text: string): number | null {
  const hex = PROGRAM_HEX.exec(text)?.[1];
  const dec = PROGRAM_DEC.exec(text)?.[1];

  return hex ? parseInt(hex, 16) : dec ? Number(dec) : null;
}

export function errorText(error: unknown): string {
  const cause = error instanceof Error && error.cause ? ` ${errorText(error.cause)}` : "";

  return `${error instanceof Error ? error.message : String(error)}${cause}`;
}

export function userRejected(): UiError {
  return {
    kind: "user_rejected",
    title: "Signature rejected",
    message: "You rejected the request in your wallet. Nothing was sent or charged.",
    action: { label: "Try again and approve it in your wallet" },
  };
}

export function insufficientFunds(faucet: string, what: "SOL" | "USDC"): UiError {
  return {
    kind: "insufficient_funds",
    title: "Insufficient funds",
    message: `Your wallet does not have enough ${what} for this.`,
    action: { label: `Get ${what} from the faucet`, href: faucet },
  };
}

function programError(code: number): UiError {
  return {
    kind: "program_error",
    title: "The Replicata program rejected the transaction",
    message: getReplicataErrorMessage(code as ReplicataError) ?? `Program error ${code}`,
    action: { label: "Review the values and try again" },
  };
}

function unknownError(text: string): UiError {
  return { kind: "unknown", title: "Something went wrong", message: text, action: { label: "Try again" } };
}

type Matcher = { test: (text: string) => boolean; error: (text: string, faucets: Faucets) => UiError };

/** First match wins, so order matters (a rejection can also mention funds). */
const TX_MATCHERS: Matcher[] = [
  { test: (text) => REJECTED.test(text), error: () => userRejected() },
  { test: (text) => NO_TOKENS.test(text), error: (_, faucets) => insufficientFunds(faucets.usdc, "USDC") },
  { test: (text) => NO_SOL.test(text), error: (_, faucets) => insufficientFunds(faucets.sol, "SOL") },
  { test: (text) => programCode(text) !== null, error: (text) => programError(programCode(text) ?? 0) },
  { test: () => true, error: unknownError },
];

/** Map an error thrown while building / signing / sending a transaction. */
export function txError(error: unknown, faucets: Faucets): UiError {
  const text = errorText(error);
  const matcher = TX_MATCHERS.find((candidate) => candidate.test(text)) ?? TX_MATCHERS[TX_MATCHERS.length - 1];

  return matcher.error(text, faucets);
}

import { expect, test } from "vitest";

import { txError } from "@/lib/errors";
import { paymentError, thrownPaymentError, type PaidResponse } from "@/lib/x402/errors";

const faucets = { sol: "https://faucet.solana.com", usdc: "https://faucet.circle.com" };

const response = (status: number, body: PaidResponse["body"] = null, retryAfter: string | null = null) =>
  paymentError({ status, body, retryAfter }, faucets);

test("success is not an error", () => {
  expect(response(200)).toBeNull();
});

test("402 insufficient_funds → faucet link from the response", () => {
  const error = response(402, { error: "insufficient_funds", faucet: "https://faucet.example" });

  expect(error).toMatchObject({
    kind: "insufficient_funds",
    action: { href: "https://faucet.example" },
  });
});

test("402 insufficient_funds without faucet falls back to the USDC faucet", () => {
  expect(response(402, { error: "insufficient_funds" })?.action?.href).toBe(faucets.usdc);
});

test("402 settlement_failed and unknown 402 bodies", () => {
  expect(response(402, { error: "settlement_failed" })?.kind).toBe("settlement_failed");
  expect(response(402)?.kind).toBe("settlement_failed");
});

test("409 not_replicated vs payment_replayed", () => {
  expect(response(409, { error: "not_replicated" })?.kind).toBe("not_replicated");
  expect(response(409, { error: "payment_replayed" })?.kind).toBe("payment_replayed");
});

test("503 facilitator unavailable mentions Retry-After", () => {
  const error = response(503, null, "5");

  expect(error?.kind).toBe("facilitator_unavailable");
  expect(error?.action?.label).toContain("5 seconds");
});

test("404 and unexpected statuses", () => {
  expect(response(404)?.kind).toBe("not_found");
  expect(response(500, { error: "boom" })).toMatchObject({
    kind: "unknown",
    message: expect.stringContaining("boom"),
  });
});

test("wallet rejection thrown by @x402/fetch", () => {
  const error = new Error("Failed to create payment payload: User rejected the request.");

  expect(thrownPaymentError(error, faucets).kind).toBe("user_rejected");
});

test("network failure thrown by fetch", () => {
  expect(thrownPaymentError(new TypeError("Failed to fetch"), faucets).kind).toBe("network");
});

test.each([
  ["WalletSignTransactionError: User rejected the request", "user_rejected"],
  [
    "Transaction simulation failed: Attempt to debit an account but found no record of a prior credit.",
    "insufficient_funds",
  ],
  ["Error processing Instruction 0: custom program error: 0x1", "insufficient_funds"],
  ["Error processing Instruction 0: custom program error: 0x1773", "program_error"],
  ["something odd", "unknown"],
])("txError(%j) → %s", (message, kind) => {
  expect(txError(new Error(message), faucets).kind).toBe(kind);
});

test("program errors use the IDL message", () => {
  const error = txError(new Error("custom program error: 0x1773"), faucets);

  expect(error.message).toBe("Deadlines must satisfy now < commit_deadline < reveal_deadline");
});

test("token-program insufficient funds points to the USDC faucet, fees to the SOL faucet", () => {
  expect(txError(new Error("custom program error: 0x1"), faucets).action?.href).toBe(faucets.usdc);
  expect(txError(new Error("Insufficient funds for fee"), faucets).action?.href).toBe(faucets.sol);
});

test("a spend-control refusal is not mistaken for a wallet rejection", () => {
  const error = new Error("Failed to create payment payload: All payment requirements were rejected by spendControls");

  expect(thrownPaymentError(error, faucets).title).toBe("Payment request refused");
});

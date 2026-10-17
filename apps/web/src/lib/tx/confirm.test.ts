import type { Signature } from "@solana/kit";
import { expect, test } from "vitest";

import { txError } from "@/lib/errors";
import { confirmSignature, failureText, type SignatureStatus, type StatusReader } from "@/lib/tx/confirm";

const SIGNATURE = "sig" as Signature;
const FAUCETS = { sol: "https://faucet.solana.com", usdc: "https://faucet.circle.com" };

function reader(...statuses: SignatureStatus[]): StatusReader {
  return async () => statuses.shift() ?? null;
}

test("resolves once the transaction is confirmed", async () => {
  const read = reader(
    null,
    { err: null, confirmationStatus: "processed" },
    { err: null, confirmationStatus: "confirmed" }
  );

  await expect(confirmSignature(read, SIGNATURE, 10_000)).resolves.toBeUndefined();
});

test("rejects with a program error the UI can explain", async () => {
  const read = reader({ err: { InstructionError: [0, { Custom: 6004 }] }, confirmationStatus: "confirmed" });
  const error = await confirmSignature(read, SIGNATURE, 10_000).catch((e: unknown) => e);

  expect(String(error)).toContain("custom program error: 0x1774");
  expect(txError(error, FAUCETS).kind).toBe("program_error");
});

test("rejects when the transaction never lands", async () => {
  await expect(confirmSignature(reader(), SIGNATURE, 1)).rejects.toThrow("not confirmed in time");
});

test("describes non-program failures verbatim", () => {
  expect(failureText({ InsufficientFundsForFee: null })).toBe('Transaction failed: {"InsufficientFundsForFee":null}');
});

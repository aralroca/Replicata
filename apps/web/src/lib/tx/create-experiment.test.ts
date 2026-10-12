import { address, type TransactionSendingSigner } from "@solana/kit";
import { expect, test } from "vitest";

import { parseCreateExperimentInstruction } from "@/generated";
import { findExperimentPda, findTokenAccount } from "@/lib/pda";
import {
  createExperimentInstruction,
  deadlines,
  type CreateExperimentInput,
  type Economics,
} from "@/lib/tx/create-experiment";

const AUTHORITY = address("9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin");
const MINT = address("4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU");

const economics: Economics = {
  reward: 3_000_000n,
  bond: 1_000_000n,
  k: 3,
  maxClaims: 3,
  commitSecs: 600,
  revealSecs: 300,
  accessPrice: 10_000n,
  authorShareBps: 7000,
};

const signer = { address: AUTHORITY, signAndSendTransactions: async () => [] } as unknown as TransactionSendingSigner;

const input: CreateExperimentInput = {
  signer,
  rpcUrl: "",
  mint: MINT,
  nonce: 9n,
  specHash: new Uint8Array(32).fill(1),
  imageDigest: new Uint8Array(32).fill(2),
  economics,
  now: 1_000,
};

const EXPECTED_DATA = {
  nonce: 9n,
  reward: 3_000_000n,
  bond: 1_000_000n,
  k: 3,
  maxClaims: 3,
  commitDeadline: 1_600n,
  revealDeadline: 1_900n,
  accessPrice: 10_000n,
  authorShareBps: 7000,
};

async function parsedInstruction() {
  const experiment = await findExperimentPda(AUTHORITY, input.nonce);

  return { experiment, parsed: parseCreateExperimentInstruction(await createExperimentInstruction(input, experiment)) };
}

test("deadlines are relative to now: commit = now + commit, reveal = commit + reveal", () => {
  expect(deadlines(economics, 1_000)).toEqual({ commitDeadline: 1_600n, revealDeadline: 1_900n });
});

test("create_experiment carries the args", async () => {
  const { parsed } = await parsedInstruction();

  expect(parsed.data).toMatchObject(EXPECTED_DATA);
  expect(Array.from(parsed.data.specHash)).toEqual(Array(32).fill(1));
});

test("create_experiment derives experiment, vault and author token accounts", async () => {
  const { experiment, parsed } = await parsedInstruction();

  expect(parsed.accounts.experiment.address).toBe(experiment);
  expect(parsed.accounts.vault.address).toBe(await findTokenAccount(experiment, MINT));
  expect(parsed.accounts.authorityTokenAccount.address).toBe(await findTokenAccount(AUTHORITY, MINT));
  expect(parsed.accounts.authority.address).toBe(AUTHORITY);
});

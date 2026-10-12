import { address, getAddressEncoder, getProgramDerivedAddress, getU64Encoder, type Address } from "@solana/kit";
import { expect, test } from "vitest";

import { REPLICATA_PROGRAM_ADDRESS } from "@/generated";
import { findExperimentPda, findTokenAccount, randomNonce } from "@/lib/pda";

const AUTHORITY = address("9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin");
const MINT = address("4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU");
const TOKEN_PROGRAM = address("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
const ATA_PROGRAM = address("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

type Seeds = Parameters<typeof getProgramDerivedAddress>[0]["seeds"];

async function expectedPda(programAddress: Address, seeds: Seeds) {
  const [pda] = await getProgramDerivedAddress({ programAddress, seeds });

  return pda;
}

/** Independent little-endian u64 encoding (not kit's), to cross-check the seed bytes. */
function u64Le(value: bigint): Uint8Array {
  return Uint8Array.from({ length: 8 }, (_, i) => Number((value >> BigInt(8 * i)) & 0xffn));
}

test("program id matches Anchor.toml / the IDL", () => {
  expect(REPLICATA_PROGRAM_ADDRESS).toBe("G9Ssnp8rjg7gpi4Sm3EZLoC73z9pdXd9etJzUDH39Wj9");
});

test("experiment PDA = ['experiment', authority, nonce u64 LE]", async () => {
  const nonce = 1_760_000_000_123n;
  const nonceLe = u64Le(nonce);
  const seeds = [new TextEncoder().encode("experiment"), getAddressEncoder().encode(AUTHORITY), nonceLe];

  expect(Array.from(getU64Encoder().encode(nonce))).toEqual(Array.from(nonceLe));
  expect(await findExperimentPda(AUTHORITY, nonce)).toBe(await expectedPda(REPLICATA_PROGRAM_ADDRESS, seeds));
});

test("different nonces give different experiments", async () => {
  expect(await findExperimentPda(AUTHORITY, 1n)).not.toBe(await findExperimentPda(AUTHORITY, 2n));
});

test("vault = ATA(experiment, mint) with the classic token program", async () => {
  const experiment = await findExperimentPda(AUTHORITY, 7n);
  const encode = (value: Address) => getAddressEncoder().encode(value) as Uint8Array;
  const seeds = [encode(experiment), encode(TOKEN_PROGRAM), encode(MINT)];

  expect(await findTokenAccount(experiment, MINT)).toBe(await expectedPda(ATA_PROGRAM, seeds));
});

test("random nonces are u64 and differ", () => {
  const [a, b] = [randomNonce(), randomNonce()];

  expect(a).not.toBe(b);
  expect(a >= 0n && a < 2n ** 64n).toBe(true);
});

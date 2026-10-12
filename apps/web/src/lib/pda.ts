// PDAs of the Replicata program (programs/replicata): experiment = ["experiment", authority, nonce u64 LE];
// vault = ATA(experiment, mint) with the classic SPL Token program.
import { getAddressEncoder, getProgramDerivedAddress, getU64Encoder, type Address } from "@solana/kit";
import { findAssociatedTokenPda, TOKEN_PROGRAM_ADDRESS } from "@solana-program/token";

import { REPLICATA_PROGRAM_ADDRESS } from "@/generated";

const EXPERIMENT_SEED = new TextEncoder().encode("experiment");

export async function findExperimentPda(
  authority: Address,
  nonce: bigint,
  programAddress: Address = REPLICATA_PROGRAM_ADDRESS
): Promise<Address> {
  const seeds = [EXPERIMENT_SEED, getAddressEncoder().encode(authority), getU64Encoder().encode(nonce)];
  const [pda] = await getProgramDerivedAddress({ programAddress, seeds });

  return pda;
}

export async function findTokenAccount(owner: Address, mint: Address): Promise<Address> {
  const [ata] = await findAssociatedTokenPda({ owner, mint, tokenProgram: TOKEN_PROGRAM_ADDRESS });

  return ata;
}

/** Random u64 nonce (wallet + nonce must be unique per experiment). */
export function randomNonce(): bigint {
  const words = crypto.getRandomValues(new Uint32Array(2));

  return (BigInt(words[0]) << 32n) | BigInt(words[1]);
}

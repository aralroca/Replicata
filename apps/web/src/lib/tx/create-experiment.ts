// Build, sign (wallet) and send `create_experiment` with the Codama client.
import {
  appendTransactionMessageInstruction,
  createSolanaRpc,
  createTransactionMessage,
  getBase58Decoder,
  pipe,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
  signAndSendTransactionMessageWithSigners,
  type Address,
  type Instruction,
  type TransactionSendingSigner,
} from "@solana/kit";

import { getCreateExperimentInstructionAsync } from "@/generated";
import { findExperimentPda, findTokenAccount } from "@/lib/pda";

export type Economics = {
  reward: bigint;
  bond: bigint;
  k: number;
  maxClaims: number;
  commitSecs: number;
  revealSecs: number;
  accessPrice: bigint;
  authorShareBps: number;
};

export type CreateExperimentInput = {
  signer: TransactionSendingSigner;
  rpcUrl: string;
  mint: Address;
  nonce: bigint;
  specHash: Uint8Array;
  imageDigest: Uint8Array;
  economics: Economics;
  now?: number;
};

export function deadlines(economics: Economics, now = Math.floor(Date.now() / 1000)) {
  const commitDeadline = BigInt(now + economics.commitSecs);

  return { commitDeadline, revealDeadline: commitDeadline + BigInt(economics.revealSecs) };
}

function instructionArgs(input: CreateExperimentInput) {
  const { economics: e } = input;

  return {
    nonce: input.nonce,
    specHash: input.specHash,
    imageDigest: input.imageDigest,
    ...{ reward: e.reward, bond: e.bond, accessPrice: e.accessPrice },
    ...{ k: e.k, maxClaims: e.maxClaims, authorShareBps: e.authorShareBps },
    ...deadlines(e, input.now),
  };
}

export async function createExperimentInstruction(input: CreateExperimentInput, experiment: Address) {
  const { signer, mint } = input;
  const authorityTokenAccount = await findTokenAccount(signer.address, mint);

  return getCreateExperimentInstructionAsync({
    ...{ authority: signer, experiment, mint, authorityTokenAccount },
    ...instructionArgs(input),
  });
}

async function transactionMessage(input: CreateExperimentInput, instruction: Instruction) {
  const { value: blockhash } = await createSolanaRpc(input.rpcUrl).getLatestBlockhash().send();

  return pipe(
    createTransactionMessage({ version: 0 }),
    (m) => setTransactionMessageFeePayerSigner(input.signer, m),
    (m) => setTransactionMessageLifetimeUsingBlockhash(blockhash, m),
    (m) => appendTransactionMessageInstruction(instruction, m)
  );
}

export async function sendCreateExperiment(input: CreateExperimentInput) {
  const experiment = await findExperimentPda(input.signer.address, input.nonce);
  const instruction = await createExperimentInstruction(input, experiment);
  const signature = await signAndSendTransactionMessageWithSigners(await transactionMessage(input, instruction));

  return { experiment, signature: getBase58Decoder().decode(signature) };
}

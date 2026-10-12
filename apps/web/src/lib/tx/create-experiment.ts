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

export async function createExperimentInstruction(input: CreateExperimentInput, experiment: Address) {
  const { economics: e, signer, mint } = input;
  const authorityTokenAccount = await findTokenAccount(signer.address, mint);
  const args = { nonce: input.nonce, specHash: input.specHash, imageDigest: input.imageDigest };
  const amounts = { reward: e.reward, bond: e.bond, accessPrice: e.accessPrice };
  const params = { k: e.k, maxClaims: e.maxClaims, authorShareBps: e.authorShareBps };
  const accounts = { authority: signer, experiment, mint, authorityTokenAccount };

  return getCreateExperimentInstructionAsync({
    ...accounts,
    ...args,
    ...amounts,
    ...params,
    ...deadlines(e, input.now),
  });
}

export async function sendCreateExperiment(
  input: CreateExperimentInput
): Promise<{ experiment: Address; signature: string }> {
  const experiment = await findExperimentPda(input.signer.address, input.nonce);
  const instruction = await createExperimentInstruction(input, experiment);
  const { value: blockhash } = await createSolanaRpc(input.rpcUrl).getLatestBlockhash().send();
  const message = pipe(
    createTransactionMessage({ version: 0 }),
    (m) => setTransactionMessageFeePayerSigner(input.signer, m),
    (m) => setTransactionMessageLifetimeUsingBlockhash(blockhash, m),
    (m) => appendTransactionMessageInstruction(instruction, m)
  );
  const signature = await signAndSendTransactionMessageWithSigners(message);

  return { experiment, signature: getBase58Decoder().decode(signature) };
}

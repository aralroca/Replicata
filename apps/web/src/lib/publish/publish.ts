// Publish = upload the spec to the coordinator, then sign + send `create_experiment`.
import { address, signature, type TransactionSendingSigner } from "@solana/kit";

import { ApiError, postSpec } from "@/lib/api";
import { RPC_URL } from "@/lib/env";
import { txError, type UiError } from "@/lib/errors";
import { bytesToHex, imageDigestBytes } from "@/lib/hex";
import { randomNonce } from "@/lib/pda";
import { specHashBytes } from "@/lib/spec/canonical";
import { confirmSignature, rpcStatusReader, type StatusReader } from "@/lib/tx/confirm";
import { sendCreateExperiment, type Economics } from "@/lib/tx/create-experiment";
import type { CoordinatorConfig, JobSpec } from "@/lib/types";

export type PublishStage = "uploading" | "signing" | "confirming";

export type PublishInput = {
  spec: JobSpec;
  economics: Economics;
  signer: TransactionSendingSigner;
  config: CoordinatorConfig;
  onStage: (stage: PublishStage) => void;
};

async function uploadSpec(spec: JobSpec): Promise<Uint8Array> {
  const local = await specHashBytes(spec);
  const remote = await postSpec(spec);

  if (remote !== bytesToHex(local)) {
    throw new Error(`spec hash mismatch: coordinator ${remote}, browser ${bytesToHex(local)}`);
  }

  return local;
}

/** Caller shows the "uploading" stage; we report signing and on-chain confirmation. */
export async function publishExperiment(input: PublishInput, read: StatusReader = rpcStatusReader(RPC_URL)) {
  const specHash = await uploadSpec(input.spec);

  input.onStage("signing");
  const sent = await sendCreateExperiment({
    ...{ signer: input.signer, rpcUrl: RPC_URL, mint: address(input.config.mint), nonce: randomNonce() },
    ...{ specHash, imageDigest: imageDigestBytes(input.spec.image_digest), economics: input.economics },
  });

  input.onStage("confirming");
  await confirmSignature(read, signature(sent.signature));

  return sent;
}

function specRejected(error: ApiError): UiError {
  return {
    kind: "unknown",
    title: "The coordinator rejected the spec",
    message: error.message,
    action: { label: "Go back to step 1 and fix it" },
  };
}

export function publishError(error: unknown, config: CoordinatorConfig): UiError {
  return error instanceof ApiError ? specRejected(error) : txError(error, config.faucets);
}

// Waits for a sent transaction to be confirmed, so the UI never moves on (e.g. to the new
// experiment's page) when the transaction failed or never landed.
import { createSolanaRpc, type Signature } from "@solana/kit";

export type SignatureStatus = { err: unknown; confirmationStatus: string | null } | null;
export type StatusReader = (signature: Signature) => Promise<SignatureStatus>;

const POLL_MS = 1_000;
const TIMEOUT_MS = 90_000;

export function rpcStatusReader(rpcUrl: string): StatusReader {
  const rpc = createSolanaRpc(rpcUrl);

  return async (signature) => {
    const { value } = await rpc.getSignatureStatuses([signature]).send();

    return value[0] as SignatureStatus;
  };
}

/** `{ InstructionError: [0, { Custom: 6004 }] }` → the text `errors.ts` already understands. */
export function failureText(err: unknown): string {
  const custom = JSON.stringify(err).match(/"Custom":(\d+)/)?.[1];

  return custom
    ? `Transaction failed: custom program error: 0x${Number(custom).toString(16)}`
    : `Transaction failed: ${JSON.stringify(err)}`;
}

function settled(status: SignatureStatus): boolean {
  if (status?.err) throw new Error(failureText(status.err));

  return status?.confirmationStatus === "confirmed" || status?.confirmationStatus === "finalized";
}

export async function confirmSignature(read: StatusReader, signature: Signature, timeoutMs = TIMEOUT_MS) {
  const deadline = Date.now() + timeoutMs;

  while (Date.now() < deadline) {
    if (settled(await read(signature))) return;
    await new Promise((resolve) => setTimeout(resolve, Math.min(POLL_MS, timeoutMs)));
  }
  throw new Error("The transaction was not confirmed in time. Check your wallet's activity and try again.");
}

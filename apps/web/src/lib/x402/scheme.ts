// `@x402/svm`'s ExactSvmScheme only accepts mainnet/devnet/testnet CAIP-2 ids. Our localnet has
// its own genesis-hash id (from GET /v1/config), so we run the scheme against an equivalent
// devnet-labelled copy of the requirements and point it at our RPC. The payload the server
// receives still carries the original `accepted` requirements (x402Client adds them).
import type { TransactionSigner } from "@solana/kit";
import { x402Client } from "@x402/core/client";
import type { PaymentRequirements, SchemeNetworkClient } from "@x402/core/types";
import { ExactSvmScheme } from "@x402/svm/exact/client";

export const SOLANA_DEVNET_CAIP2 = "solana:EtWTRABZaYq6iMfeYKouRu166VU2xqa1";

const KNOWN_NETWORKS = [
  "solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp",
  SOLANA_DEVNET_CAIP2,
  "solana:4uhcVJyU9pJkvQyS88uRDiswHXSCkY3z",
];

export function schemeNetwork(network: string): string {
  return KNOWN_NETWORKS.includes(network) ? network : SOLANA_DEVNET_CAIP2;
}

export function anyNetworkSvmScheme(signer: TransactionSigner, rpcUrl: string): SchemeNetworkClient {
  const inner = new ExactSvmScheme(signer, { rpcUrl });

  return {
    scheme: inner.scheme,
    createPaymentPayload: (version: number, requirements: PaymentRequirements) =>
      inner.createPaymentPayload(version, {
        ...requirements,
        network: schemeNetwork(requirements.network) as PaymentRequirements["network"],
      }),
  };
}

/** x402 client that pays `network` (CAIP-2 from /v1/config) with the connected wallet. */
export function createPaymentClient(signer: TransactionSigner, network: string, rpcUrl: string) {
  return new x402Client().register(
    network as PaymentRequirements["network"],
    anyNetworkSvmScheme(signer, rpcUrl)
  );
}

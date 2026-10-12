import { generateKeyPairSigner } from "@solana/kit";
import type { PaymentRequirements } from "@x402/core/types";
import { ExactSvmScheme } from "@x402/svm/exact/client";
import { afterEach, expect, test, vi } from "vitest";

import { filenameFrom } from "@/lib/x402/pay";
import { anyNetworkSvmScheme, schemeNetwork, SOLANA_DEVNET_CAIP2 } from "@/lib/x402/scheme";

const LOCALNET = "solana:Fte3CqCT5A5K9Lb88mvQfc9ULeSuamKZ";
const MAINNET = "solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp";

const requirements = { scheme: "exact", network: LOCALNET, amount: "10000", payTo: "x", asset: "y" };

afterEach(() => {
  vi.restoreAllMocks();
});

test("known networks pass through, any other solana network is mapped to devnet", () => {
  expect(schemeNetwork(SOLANA_DEVNET_CAIP2)).toBe(SOLANA_DEVNET_CAIP2);
  expect(schemeNetwork(MAINNET)).toBe(MAINNET);
  expect(schemeNetwork(LOCALNET)).toBe(SOLANA_DEVNET_CAIP2);
});

test("the adapter hands ExactSvmScheme a supported network and keeps everything else", async () => {
  const payload = { x402Version: 2, payload: {} };
  const spy = vi.spyOn(ExactSvmScheme.prototype, "createPaymentPayload").mockResolvedValue(payload);
  const scheme = anyNetworkSvmScheme(await generateKeyPairSigner(), "http://127.0.0.1:8899");

  await scheme.createPaymentPayload(2, requirements as unknown as PaymentRequirements);

  expect(scheme.scheme).toBe("exact");
  expect(spy).toHaveBeenCalledWith(2, { ...requirements, network: SOLANA_DEVNET_CAIP2 });
});

test("download filename comes from Content-Disposition", () => {
  expect(filenameFrom('attachment; filename="abc.tar.gz"', "x.tar.gz")).toBe("abc.tar.gz");
  expect(filenameFrom(null, "x.tar.gz")).toBe("x.tar.gz");
});

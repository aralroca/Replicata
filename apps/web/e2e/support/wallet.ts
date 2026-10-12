// Node side of the injected Wallet Standard wallet: key material + page wiring.
import { generateKeyPairSync } from "node:crypto";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import type { Page } from "@playwright/test";
import { getBase58Decoder } from "@solana/kit";

import { isReal, RPC_URL } from "./env";

export type WalletConfig = { seed: number[]; publicKey: number[]; address: string; sendRpc?: string };

const SCRIPT = fileURLToPath(new URL("./mock-wallet.js", import.meta.url));

function freshKey(): { seed: number[]; publicKey: number[] } {
  const { privateKey, publicKey } = generateKeyPairSync("ed25519");
  const pkcs8 = privateKey.export({ format: "der", type: "pkcs8" });
  const spki = publicKey.export({ format: "der", type: "spki" });

  return { seed: Array.from(pkcs8.subarray(-32)), publicKey: Array.from(spki.subarray(-32)) };
}

/** Solana CLI keypair JSON: 64 bytes = seed ‖ public key. */
function keypairFile(path: string): { seed: number[]; publicKey: number[] } {
  const bytes: number[] = JSON.parse(readFileSync(path, "utf8"));

  return { seed: bytes.slice(0, 32), publicKey: bytes.slice(32, 64) };
}

export function walletConfig(): WalletConfig {
  const keypair = process.env.E2E_WALLET_KEYPAIR ? keypairFile(process.env.E2E_WALLET_KEYPAIR) : freshKey();
  const address = getBase58Decoder().decode(Uint8Array.from(keypair.publicKey));

  return { ...keypair, address, sendRpc: isReal ? RPC_URL : undefined };
}

export async function installWallet(page: Page, config: WalletConfig): Promise<void> {
  await page.addInitScript((value) => Object.assign(window, { __E2E_WALLET__: value }), config);
  await page.addInitScript({ path: SCRIPT });
}

export async function connectWallet(page: Page): Promise<void> {
  await page.getByRole("button", { name: "Select Wallet" }).click();
  await page.getByRole("menuitem", { name: /E2E Wallet/ }).click();
}

export function rejectNextSignature(page: Page): Promise<void> {
  return page.evaluate(() => {
    (window as unknown as { __e2eWallet: { rejectNext: boolean } }).__e2eWallet.rejectNext = true;
  });
}

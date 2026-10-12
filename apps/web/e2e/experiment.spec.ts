import type { APIRequestContext, Page } from "@playwright/test";
import { getTransactionDecoder } from "@solana/kit";

import { FEE_PAYER } from "./mock/fixtures.mjs";

import { COORDINATOR_URL, isReal, SCENARIOS } from "./support/env";
import { errorAlert, expect, test } from "./support/test";
import { connectWallet, rejectNextSignature } from "./support/wallet";

const RESULT_BUTTON = /Get verified result \(\$[\d,]+\.\d+\)/;

/** Which accounts already signed a base64 wire transaction (null = slot left for the facilitator). */
function signers(base64: string): Record<string, boolean> {
  const { signatures } = getTransactionDecoder().decode(Buffer.from(base64, "base64"));

  return Object.fromEntries(Object.entries(signatures).map(([key, signature]) => [key, signature !== null]));
}

/** Last PAYMENT-SIGNATURE payload the mock coordinator received. */
async function lastPayment(request: APIRequestContext) {
  const payments = await (await request.get(`${COORDINATOR_URL}/__payments`)).json();

  return payments.at(-1);
}

/** The x402 payload targets our localnet network/PDA and only the wallet has signed (facilitator pays fees). */
async function expectWalletPayment(request: APIRequestContext, walletAddress: string) {
  const payment = await lastPayment(request);

  expect(payment.accepted).toMatchObject({
    network: "solana:Fte3CqCT5A5K9Lb88mvQfc9ULeSuamKZ",
    payTo: SCENARIOS.noFunds,
  });
  expect(signers(payment.payload.transaction)).toEqual({ [FEE_PAYER]: false, [walletAddress]: true });
}

async function openAndPay(page: Page, id: string | undefined) {
  test.skip(!id, "no experiment id for this scenario");
  await page.goto(`/e/${id}`);
  await connectWallet(page);
  await page.getByRole("button", { name: RESULT_BUTTON }).click();
}

test("timeline shows claims → commits → reveals → finalize", async ({ page }) => {
  test.skip(!SCENARIOS.revealing || isReal, "needs the mock's revealing experiment");
  await page.goto(`/e/${SCENARIOS.revealing}`);

  await expect(page.getByTestId("step-claims")).toHaveAttribute("data-state", "done");
  await expect(page.getByTestId("step-commits")).toHaveAttribute("data-state", "done");
  await expect(page.getByTestId("step-reveals")).toHaveAttribute("data-state", "active");
  await expect(page.getByTestId("step-reveals")).toContainText("1/3");
  await expect(page.getByTestId("step-finalize")).toHaveAttribute("data-state", "pending");
});

test("replicate box and badge snippet", async ({ page }) => {
  test.skip(!SCENARIOS.paid, "no replicated experiment id");
  await page.goto(`/e/${SCENARIOS.paid}`);

  await expect(page.getByLabel("Replicate command")).toContainText(`/install.sh | sh && replicata run`);
  await expect(page.getByLabel("Replicate command")).toContainText(`--once ${SCENARIOS.paid}`);
  await expect(page.getByLabel("Badge markdown")).toContainText(`/e/${SCENARIOS.paid}/badge.svg`);
  await expect(page.getByRole("img", { name: "Replication status badge" })).toBeVisible();
});

test("result button needs a wallet", async ({ page }) => {
  test.skip(!SCENARIOS.paid, "no replicated experiment id");
  await page.goto(`/e/${SCENARIOS.paid}`);

  await expect(page.getByRole("button", { name: RESULT_BUTTON })).toBeDisabled();
});

test("409: not replicated yet, nothing charged", async ({ page }) => {
  await openAndPay(page, SCENARIOS.revealing);

  await expect(errorAlert(page)).toContainText("Not replicated yet");
  await expect(errorAlert(page)).toContainText("You were not charged");
});

test("402: insufficient USDC links the faucet", async ({ page }) => {
  await openAndPay(page, SCENARIOS.noFunds);

  await expect(errorAlert(page)).toContainText("Insufficient funds");
  await expect(page.getByRole("link", { name: /Get USDC from the faucet/ })).toHaveAttribute("href", /faucet/);
});

test("the payment is partially signed by the wallet for the experiment's network", async ({
  page,
  request,
  wallet,
}) => {
  test.skip(isReal, "inspects the mock's received payments");
  await openAndPay(page, SCENARIOS.noFunds);
  await expect(errorAlert(page)).toBeVisible();
  await expectWalletPayment(request, wallet.address);
});

test("503: facilitator down suggests retrying", async ({ page }) => {
  await openAndPay(page, SCENARIOS.down);

  await expect(errorAlert(page)).toContainText("Payment service unavailable");
  await expect(errorAlert(page)).toContainText("Retry in 5 seconds");
});

test("rejecting the payment signature is explained", async ({ page }) => {
  test.skip(!SCENARIOS.paid, "no replicated experiment id");
  await page.goto(`/e/${SCENARIOS.paid}`);
  await connectWallet(page);
  await rejectNextSignature(page);
  await page.getByRole("button", { name: RESULT_BUTTON }).click();

  await expect(errorAlert(page)).toContainText("Signature rejected");
});

test("paying downloads the verified bundle", async ({ page }) => {
  const download = page.waitForEvent("download");

  await openAndPay(page, SCENARIOS.paid);

  expect((await download).suggestedFilename()).toBe(`${SCENARIOS.paid}.tar.gz`);
  await expect(page.getByRole("status").filter({ hasText: "Downloaded" })).toBeVisible();
});

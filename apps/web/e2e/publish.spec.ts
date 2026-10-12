import type { Page } from "@playwright/test";

import { isReal } from "./support/env";
import { errorAlert, expect, test } from "./support/test";
import { connectWallet, rejectNextSignature } from "./support/wallet";

const DIGEST = `sha256:${"0f".repeat(32)}`;

async function fillValidSpec(page: Page) {
  await page.getByLabel("Title").fill(`E2E π ${Date.now()}`);
  await page.getByLabel("Image repository").fill("ghcr.io/aralroca/replicata-pi");
  await page.getByLabel("Image digest").fill(DIGEST);
  await page.getByLabel("Command").fill("python\n/app/main.py");
}

async function goToReview(page: Page) {
  await page.goto("/publish");
  await fillValidSpec(page);
  await page.getByRole("button", { name: "Next" }).click();
  await page.getByRole("button", { name: "Next" }).click();
  await expect(page.getByRole("heading", { name: "3 · Review & publish" })).toBeVisible();
}

test("step 1 shows JobSpec::validate errors and blocks Next", async ({ page }) => {
  const issues = page.getByTestId("issues");

  await page.goto("/publish");
  await page.getByRole("button", { name: "Next" }).click();

  await expect(issues).toContainText("title must be 1..=120 characters");
  await expect(issues).toContainText("image_digest must look like sha256:<64 hex chars>");
  await expect(issues).toContainText("cmd must not be empty");
  await expect(page.getByLabel("Title")).toHaveAttribute("aria-invalid", "true");
  await expect(page.getByText("Step 1 of 3")).toBeVisible();
});

test("step 1 rejects a tagged image, bad input lines and resources", async ({ page }) => {
  const issues = page.getByTestId("issues");

  await page.goto("/publish");
  await fillValidSpec(page);
  await page.getByLabel("Image repository").fill("alpine@sha256");
  await page.getByLabel("Inputs").fill("ftp://x");
  await page.getByLabel("Memory (MB)").fill("1");
  await page.getByRole("button", { name: "Next" }).click();

  await expect(issues).toContainText("image must be a non-empty repository name without tag or digest");
  await expect(issues).toContainText("inputs: one `<url> <sha256> <path>` per line");
  await expect(issues).toContainText("resources out of range");
});

test("step 2 enforces 1 ≤ k ≤ slots ≤ 5 and a positive reward", async ({ page }) => {
  await page.goto("/publish");
  await fillValidSpec(page);
  await page.getByRole("button", { name: "Next" }).click();
  await page.getByLabel("Required matches (k)").fill("4");
  await page.getByLabel("Reward (USDC)").fill("0");
  await page.getByRole("button", { name: "Next" }).click();

  await expect(page.getByTestId("issues")).toContainText("Need 1 ≤ k ≤ max claims ≤ 5");
  await expect(page.getByTestId("issues")).toContainText("Reward must be greater than zero");
});

test("review asks to connect a wallet first", async ({ page }) => {
  await goToReview(page);

  await expect(page.getByText("Connect a wallet (top right) to publish.")).toBeVisible();
  await expect(page.getByLabel("Canonical spec JSON")).toContainText('"version":1');
});

test("publishing uploads the spec, signs create_experiment and opens the experiment", async ({ page }) => {
  await goToReview(page);
  await connectWallet(page);
  await page.getByRole("button", { name: "Publish & escrow reward" }).click();

  await expect(page).toHaveURL(/\/e\/[1-9A-HJ-NP-Za-km-z]{32,44}$/, { timeout: isReal ? 60_000 : 15_000 });
  expect(await page.evaluate(() => (window as unknown as { __e2eWallet: { signed: number } }).__e2eWallet.signed)).toBe(
    1
  );
});

test("a rejected signature is explained and nothing is sent", async ({ page }) => {
  await goToReview(page);
  await connectWallet(page);
  await rejectNextSignature(page);
  await page.getByRole("button", { name: "Publish & escrow reward" }).click();

  await expect(errorAlert(page)).toContainText("Signature rejected");
  await expect(errorAlert(page)).toContainText("Try again and approve it in your wallet");
});

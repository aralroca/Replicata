import { isReal, SCENARIOS } from "./support/env";
import { expect, test } from "./support/test";

test("landing: pitch, CTA and the experiment feed", async ({ page }) => {
  await page.goto("/");

  await expect(page.getByRole("heading", { level: 1 })).toContainText("Results you can verify");
  await expect(page.getByRole("link", { name: "Publish an experiment" })).toBeVisible();
  await expect(page.getByTestId("experiment-card").first()).toBeVisible();
});

test("feed shows status/phase badges, reward and k", async ({ page }) => {
  const card = page.getByTestId("experiment-card").filter({ hasText: "Monte-Carlo π (in progress)" });

  test.skip(isReal, "exact fixtures only exist in the mock");
  await page.goto("/");

  await expect(page.getByTestId("experiment-card")).toHaveCount(4);
  await expect(card).toContainText("Open");
  await expect(card).toContainText("Revealing");
  await expect(card).toContainText("3.00 USDC");
  await expect(card).toContainText("1/3");
});

test("a feed card opens its experiment page", async ({ page }) => {
  test.skip(isReal, "exact fixtures only exist in the mock");
  await page.goto("/");
  await page.getByRole("link", { name: "Cellular automaton rule 110" }).click();

  await expect(page).toHaveURL(new RegExp(`/e/${SCENARIOS.paid}$`));
  await expect(page.getByRole("heading", { level: 1 })).toHaveText("Cellular automaton rule 110");
});

test("worker profile shows reputation and earnings", async ({ page }) => {
  test.skip(isReal, "exact fixtures only exist in the mock");
  await page.goto("/w/5H8cfQiAYbf8Pmn2Sq17x5Y4S4wz34cSW1MXSsFJ4Cbe");

  await expect(page.getByRole("heading", { name: "Worker" })).toBeVisible();
  await expect(page.getByText("67%")).toBeVisible();
  await expect(page.getByText("2.00 USDC")).toBeVisible();
});

test("researcher page shows royalties and experiments", async ({ page }) => {
  test.skip(isReal, "exact fixtures only exist in the mock");
  await page.goto("/r/6TiLSzsZbHT8AziLjYXf8foiL5Zk3MymhUMaWBXG1vCT");

  await expect(page.getByRole("heading", { name: "Researcher" })).toBeVisible();
  await expect(page.getByText("0.06 USDC")).toBeVisible();
  await expect(page.getByTestId("experiment-card")).toHaveCount(4);
});

test("onboarding links the faucets from /v1/config", async ({ page }) => {
  await page.goto("/start");

  await expect(page.getByRole("link", { name: "SOL faucet" })).toHaveAttribute("href", /faucet/);
  await expect(page.getByRole("link", { name: "USDC faucet" })).toHaveAttribute("href", /faucet/);
});

test("agents docs link the OpenAPI document", async ({ page }) => {
  await page.goto("/docs");

  await expect(page.getByRole("link", { name: /openapi\.json$/ })).toBeVisible();
  await expect(page.getByText("wrapFetchWithPayment").first()).toBeVisible();
});

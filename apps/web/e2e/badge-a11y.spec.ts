import { isReal, SCENARIOS } from "./support/env";
import { errorAlert, expect, seriousViolations, test } from "./support/test";
import { connectWallet } from "./support/wallet";

test("badge.svg for a replicated experiment", async ({ request }) => {
  test.skip(!SCENARIOS.paid, "no replicated experiment id");
  const response = await request.get(`/e/${SCENARIOS.paid}/badge.svg`);

  expect(response.status()).toBe(200);
  expect(response.headers()["content-type"]).toContain("image/svg+xml");
  expect(await response.text()).toMatch(/Replicated ✅ \d\/\d/);
});

test("badge.svg while replicating and for unknown ids", async ({ request }) => {
  test.skip(isReal, "uses mock fixtures");
  const replicating = await request.get(`/e/${SCENARIOS.revealing}/badge.svg`);
  const unknown = await request.get("/e/11111111111111111111111111111111/badge.svg");

  expect(await replicating.text()).toContain("Replicating 3/3");
  expect(await unknown.text()).toContain("not found");
});

const A11Y_PATHS = ["/", "/publish", `/e/${SCENARIOS.paid ?? SCENARIOS.revealing ?? ""}`];

A11Y_PATHS.forEach((path) =>
  test(`no serious axe violations on ${path}`, async ({ page }) => {
    await page.goto(path);
    await page.waitForLoadState("networkidle");

    expect(await seriousViolations(page)).toEqual([]);
  })
);

test("no serious axe violations on an experiment page with an error shown", async ({ page }) => {
  test.skip(!SCENARIOS.revealing, "no not-replicated experiment id");
  await page.goto(`/e/${SCENARIOS.revealing}`);
  await connectWallet(page);
  await page.getByRole("button", { name: /Get verified result/ }).click();
  await expect(errorAlert(page)).toBeVisible();

  expect(await seriousViolations(page)).toEqual([]);
});

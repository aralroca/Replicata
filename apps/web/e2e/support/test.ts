// `test` with the E2E wallet injected into every page and a11y helper.
import AxeBuilder from "@axe-core/playwright";
import { test as base, expect, type Page } from "@playwright/test";

import { installWallet, walletConfig, type WalletConfig } from "./wallet";

export const test = base.extend<{ wallet: WalletConfig }>({
  wallet: async ({}, provide) => {
    await provide(walletConfig());
  },
  page: async ({ page, wallet }, provide) => {
    await installWallet(page, wallet);
    await provide(page);
  },
});

export { expect };

/** Our error callouts (Next's route announcer is also role="alert"). */
export function errorAlert(page: Page) {
  return page.locator('[role="alert"][data-kind]');
}

/** axe violations with impact serious/critical (WCAG 2.x A/AA rules). */
export async function seriousViolations(page: Page) {
  const results = await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"]).analyze();

  return results.violations
    .filter((violation) => violation.impact === "serious" || violation.impact === "critical")
    .map((violation) => ({ id: violation.id, nodes: violation.nodes.map((node) => node.target.join(" ")) }));
}

// E2E: by default against e2e/mock/server.mjs (coordinator + RPC mock). Set E2E_REAL=1 plus the
// NEXT_PUBLIC_* URLs of a running localnet + coordinator to run the same specs for real.
import { defineConfig, devices } from "@playwright/test";

import { COORDINATOR_URL, isReal, MOCK_PORT, RPC_URL, WEB_PORT, WEB_URL } from "./e2e/support/env";

const appEnv = {
  NEXT_PUBLIC_CLUSTER: process.env.NEXT_PUBLIC_CLUSTER ?? "localnet",
  NEXT_PUBLIC_RPC_URL: RPC_URL,
  NEXT_PUBLIC_COORDINATOR_URL: COORDINATOR_URL,
  NEXT_PUBLIC_SITE_URL: WEB_URL,
  NEXT_DIST_DIR: ".next-e2e",
  NEXT_TELEMETRY_DISABLED: "1",
};

const mockServer = {
  command: "node e2e/mock/server.mjs",
  url: `http://localhost:${MOCK_PORT}/v1/config`,
  env: { PORT: String(MOCK_PORT) },
  reuseExistingServer: false,
};

const webServer = {
  command: `pnpm exec next build && pnpm exec next start -p ${WEB_PORT}`,
  url: WEB_URL,
  env: appEnv,
  timeout: 240_000,
  reuseExistingServer: false,
};

export default defineConfig({
  testDir: "e2e",
  fullyParallel: true,
  forbidOnly: Boolean(process.env.CI),
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? "github" : "list",
  use: { baseURL: WEB_URL, trace: "retain-on-failure", acceptDownloads: true },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
  webServer: isReal ? [webServer] : [mockServer, webServer],
});

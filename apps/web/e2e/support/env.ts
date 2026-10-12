// E2E environment. Default: mock coordinator + mock RPC (e2e/mock/server.mjs).
// `E2E_REAL=1`: no mock; point NEXT_PUBLIC_* at a real localnet + coordinator and provide
// E2E_WALLET_KEYPAIR (solana keypair JSON with SOL + USDC) and the scenario ids below.
import { IDS } from "../mock/fixtures.mjs";

export const isReal = process.env.E2E_REAL === "1";

export const WEB_PORT = Number(process.env.E2E_WEB_PORT ?? 3210);
export const MOCK_PORT = Number(process.env.E2E_MOCK_PORT ?? 8799);
export const WEB_URL = `http://localhost:${WEB_PORT}`;

const mockUrl = `http://localhost:${MOCK_PORT}`;

export const COORDINATOR_URL = process.env.NEXT_PUBLIC_COORDINATOR_URL ?? mockUrl;
export const RPC_URL = process.env.NEXT_PUBLIC_RPC_URL ?? `${mockUrl}/rpc`;

/** Experiments per scenario: fixed ids against the mock, env-provided ids against a real stack. */
export const SCENARIOS = {
  revealing: isReal ? process.env.E2E_ID_NOT_REPLICATED : IDS.revealing,
  noFunds: isReal ? process.env.E2E_ID_NO_FUNDS : IDS.noFunds,
  paid: isReal ? process.env.E2E_ID_REPLICATED : IDS.paid,
  down: isReal ? undefined : IDS.down,
};

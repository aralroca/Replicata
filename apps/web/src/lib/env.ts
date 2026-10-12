// Public env (inlined at build time by Next: keep the `process.env.NEXT_PUBLIC_*` reads literal).

export type Cluster = "devnet" | "localnet";

export const CLUSTER: Cluster = process.env.NEXT_PUBLIC_CLUSTER === "localnet" ? "localnet" : "devnet";

export const RPC_URL =
  process.env.NEXT_PUBLIC_RPC_URL ??
  (CLUSTER === "localnet" ? "http://127.0.0.1:8899" : "https://api.devnet.solana.com");

export const COORDINATOR_URL = (
  process.env.NEXT_PUBLIC_COORDINATOR_URL ?? "http://localhost:8787"
).replace(/\/$/, "");

/** Public origin of this site, used in the install one-liner and badge snippets. */
export const SITE_URL = (process.env.NEXT_PUBLIC_SITE_URL ?? "http://localhost:3000").replace(/\/$/, "");

/** Coordinator URL used by server code (route handlers); may differ inside a container. */
export function serverCoordinatorUrl(): string {
  return (process.env.COORDINATOR_URL ?? COORDINATOR_URL).replace(/\/$/, "");
}

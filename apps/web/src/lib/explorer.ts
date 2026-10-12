import { CLUSTER, RPC_URL, type Cluster } from "@/lib/env";

function clusterQuery(cluster: Cluster, rpcUrl: string): string {
  return cluster === "devnet"
    ? "?cluster=devnet"
    : `?cluster=custom&customUrl=${encodeURIComponent(rpcUrl)}`;
}

export function explorerUrl(
  kind: "address" | "tx",
  value: string,
  cluster: Cluster = CLUSTER,
  rpcUrl: string = RPC_URL
): string {
  return `https://explorer.solana.com/${kind}/${value}${clusterQuery(cluster, rpcUrl)}`;
}

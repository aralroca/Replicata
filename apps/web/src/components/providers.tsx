"use client";

import { createSolanaDevnet, createSolanaLocalnet, createWalletUiConfig, WalletUi } from "@wallet-ui/react";
import type { ReactNode } from "react";

import { CLUSTER, RPC_URL } from "@/lib/env";

const cluster = CLUSTER === "localnet" ? createSolanaLocalnet({ url: RPC_URL }) : createSolanaDevnet({ url: RPC_URL });

const config = createWalletUiConfig({ clusters: [cluster] });

export function Providers({ children }: { children: ReactNode }) {
  return <WalletUi config={config}>{children}</WalletUi>;
}

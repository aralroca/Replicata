"use client";

import Link from "next/link";
import type { ReactNode } from "react";

import { replicateCommand } from "@/components/experiment/share-boxes";
import { useConfig } from "@/hooks/use-coordinator";
import { CLUSTER } from "@/lib/env";

const DEFAULT_FAUCETS = { sol: "https://faucet.solana.com", usdc: "https://faucet.circle.com" };

function OnboardingStep({ n, title, children }: { n: number; title: string; children: ReactNode }) {
  return (
    <li className="card grid gap-2">
      <h2 className="font-semibold">
        <span className="text-accent">{n}.</span> {title}
      </h2>
      <div className="text-sm text-muted">{children}</div>
    </li>
  );
}

function WalletStep() {
  return (
    <OnboardingStep n={1} title="Install a wallet (15 s)">
      Install <a href="https://phantom.com/download" target="_blank" rel="noreferrer">Phantom</a>, then
      enable <em>Settings → Developer settings → Testnet mode</em> ({CLUSTER}). Click <em>Select Wallet</em> at the top.
    </OnboardingStep>
  );
}

function FundsStep({ faucets }: { faucets: typeof DEFAULT_FAUCETS }) {
  return (
    <OnboardingStep n={2} title="Get test funds (30 s)">
      <a href={faucets.sol} target="_blank" rel="noreferrer">SOL faucet</a> pays transaction fees;{" "}
      <a href={faucets.usdc} target="_blank" rel="noreferrer">USDC faucet</a> (Solana {CLUSTER}) pays rewards, bonds and
      result access.
    </OnboardingStep>
  );
}

function ActStep() {
  return (
    <OnboardingStep n={3} title="Publish or replicate (15 s)">
      <Link href="/publish">Publish an experiment</Link> with a reward, or earn by replicating one:
      <pre className="mono mt-2 overflow-x-auto rounded-lg bg-background p-3 text-xs text-foreground" tabIndex={0}>
        <code>{replicateCommand("<experiment-id>")}</code>
      </pre>
    </OnboardingStep>
  );
}

export function Onboarding() {
  const { data: config } = useConfig();

  return (
    <ol className="grid gap-4">
      <WalletStep />
      <FundsStep faucets={config?.faucets ?? DEFAULT_FAUCETS} />
      <ActStep />
    </ol>
  );
}

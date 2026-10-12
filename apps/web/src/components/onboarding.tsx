"use client";

import Link from "next/link";
import type { ReactNode } from "react";

import { Code, replicateCommand } from "@/components/experiment/share-boxes";
import { useConfig } from "@/hooks/use-coordinator";
import { CLUSTER } from "@/lib/env";

const DEFAULT_FAUCETS = { sol: "https://faucet.solana.com", usdc: "https://faucet.circle.com" };

const External = ({ href, children }: { href: string; children: ReactNode }) => (
  <a href={href} target="_blank" rel="noreferrer">
    {children}
  </a>
);

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
      Install <External href="https://phantom.com/download">Phantom</External>, enable{" "}
      <em>Settings → Developer settings → Testnet mode</em> ({CLUSTER}) and click <em>Select Wallet</em> at the top.
    </OnboardingStep>
  );
}

function FundsStep({ faucets }: { faucets: typeof DEFAULT_FAUCETS }) {
  return (
    <OnboardingStep n={2} title="Get test funds (30 s)">
      The <External href={faucets.sol}>SOL faucet</External> pays transaction fees; the{" "}
      <External href={faucets.usdc}>USDC faucet</External> (Solana {CLUSTER}) pays rewards, bonds and result access.
    </OnboardingStep>
  );
}

function ActStep() {
  return (
    <OnboardingStep n={3} title="Publish or replicate (15 s)">
      <Link href="/publish">Publish an experiment</Link> with a reward, or earn by replicating one:
      <Code code={replicateCommand("<experiment-id>")} label="Replicate command" />
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

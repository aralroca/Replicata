"use client";

import { WalletUiDropdown } from "@wallet-ui/react";
import Link from "next/link";

import { CLUSTER } from "@/lib/env";

const LINKS = [
  { href: "/", label: "Experiments" },
  { href: "/publish", label: "Publish" },
  { href: "/start", label: "Start" },
  { href: "/docs", label: "Agents API" },
];

function Nav() {
  return (
    <nav aria-label="Main" className="flex flex-wrap items-center gap-4 text-sm">
      {LINKS.map((link) => (
        <Link key={link.href} href={link.href} className="text-foreground no-underline hover:underline">
          {link.label}
        </Link>
      ))}
    </nav>
  );
}

function Brand() {
  return (
    <Link href="/" className="text-lg font-bold text-foreground no-underline">
      Replicata <span className="text-xs font-normal text-muted">{CLUSTER}</span>
    </Link>
  );
}

export function Header() {
  return (
    <header className="border-b border-border">
      <div className="mx-auto flex max-w-5xl flex-wrap items-center justify-between gap-4 px-4 py-3">
        <Brand />
        <Nav />
        <WalletUiDropdown />
      </div>
    </header>
  );
}

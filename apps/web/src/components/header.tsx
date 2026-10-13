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
    <nav aria-label="Main" className="flex flex-wrap items-center gap-6 text-sm font-medium">
      {LINKS.map((link) => (
        <Link key={link.href} href={link.href} className="text-foreground no-underline hover:text-accent">
          {link.label}
        </Link>
      ))}
    </nav>
  );
}

const BRAND_CLASS =
  "flex items-center gap-2 text-lg font-semibold tracking-[-0.03em] text-foreground no-underline hover:no-underline";

function Brand() {
  return (
    <Link href="/" className={BRAND_CLASS}>
      {/* eslint-disable-next-line @next/next/no-img-element -- static SVG logo */}
      <img src="/mark.svg" alt="" width={26} height={26} />
      Replicata
      <span className="rounded-full bg-subtle px-2 py-0.5 text-[11px] font-medium text-muted">{CLUSTER}</span>
    </Link>
  );
}

export function Header() {
  return (
    <header className="sticky top-0 z-20 border-b border-border bg-background/80 backdrop-blur">
      <div className="mx-auto flex h-16 max-w-6xl items-center justify-between gap-6 px-6">
        <Brand />
        <Nav />
        <WalletUiDropdown />
      </div>
    </header>
  );
}

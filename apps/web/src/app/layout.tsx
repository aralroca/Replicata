import type { Metadata } from "next";
import type { ReactNode } from "react";

import { Header } from "@/components/header";
import { Providers } from "@/components/providers";

import "./globals.css";

export const metadata: Metadata = {
  title: { default: "Replicata — verifiable, paid replication", template: "%s · Replicata" },
  description:
    "Publish a computational experiment with a USDC reward; independent workers replicate it and the verified result is sealed on Solana.",
};

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="en">
      <body>
        <Providers>
          <Header />
          <main className="mx-auto max-w-5xl px-4 py-8">{children}</main>
        </Providers>
      </body>
    </html>
  );
}

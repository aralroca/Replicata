import type { Metadata } from "next";
import { Inter } from "next/font/google";
import type { ReactNode } from "react";

import { Footer } from "@/components/footer";
import { Header } from "@/components/header";
import { Providers } from "@/components/providers";

import "./globals.css";

const inter = Inter({ subsets: ["latin"], variable: "--font-inter", display: "swap" });

export const metadata: Metadata = {
  title: { default: "Replicata — verifiable, paid replication", template: "%s · Replicata" },
  description:
    "Publish a computational experiment with a USDC reward; independent workers replicate it and the verified result is sealed on Solana.",
};

function Shell({ children }: { children: ReactNode }) {
  return (
    <Providers>
      <Header />
      <main className="mx-auto max-w-6xl px-6 py-12">{children}</main>
      <Footer />
    </Providers>
  );
}

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="en" className={inter.variable}>
      <body>
        <Shell>{children}</Shell>
      </body>
    </html>
  );
}

import type { Metadata } from "next";

import { ResearcherProfile } from "@/components/profiles";

type Props = { params: Promise<{ pubkey: string }> };

export const metadata: Metadata = { title: "Researcher" };

export default async function Page({ params }: Props) {
  const { pubkey } = await params;

  return <ResearcherProfile pubkey={pubkey} />;
}

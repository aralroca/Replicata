import type { Metadata } from "next";

import { WorkerProfile } from "@/components/profiles";

type Props = { params: Promise<{ pubkey: string }> };

export const metadata: Metadata = { title: "Worker" };

export default async function Page({ params }: Props) {
  const { pubkey } = await params;

  return <WorkerProfile pubkey={pubkey} />;
}

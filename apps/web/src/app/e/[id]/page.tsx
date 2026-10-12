import type { Metadata } from "next";

import { ExperimentPage } from "@/components/experiment/experiment-page";

type Props = { params: Promise<{ id: string }> };

export async function generateMetadata({ params }: Props): Promise<Metadata> {
  const { id } = await params;

  return { title: `Experiment ${id.slice(0, 8)}…` };
}

export default async function Page({ params }: Props) {
  const { id } = await params;

  return <ExperimentPage id={id} />;
}

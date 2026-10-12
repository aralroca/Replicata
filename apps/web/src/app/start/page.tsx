import type { Metadata } from "next";

import { Onboarding } from "@/components/onboarding";
import { PageShell } from "@/components/page-header";

export const metadata: Metadata = { title: "Start in 60 seconds" };

export default function StartPage() {
  return (
    <PageShell title="Start in 60 seconds" lead="Three steps from zero to publishing or replicating an experiment.">
      <Onboarding />
    </PageShell>
  );
}

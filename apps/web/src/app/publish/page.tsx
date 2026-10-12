import type { Metadata } from "next";

import { PageShell } from "@/components/page-header";
import { PublishWizard } from "@/components/publish/wizard";

export const metadata: Metadata = { title: "Publish an experiment" };

const LEAD =
  "Describe a deterministic container run, put a USDC reward in escrow and let independent workers replicate it.";

export default function PublishPage() {
  return (
    <PageShell title="Publish an experiment" lead={LEAD}>
      <PublishWizard />
    </PageShell>
  );
}

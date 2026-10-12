import type { Metadata } from "next";

import { PublishWizard } from "@/components/publish/wizard";

export const metadata: Metadata = { title: "Publish an experiment" };

export default function PublishPage() {
  return (
    <div className="grid gap-6">
      <header>
        <h1 className="text-2xl font-bold">Publish an experiment</h1>
        <p className="mt-2 text-muted">
          Describe a deterministic container run, put a USDC reward in escrow and let independent
          workers replicate it.
        </p>
      </header>
      <PublishWizard />
    </div>
  );
}

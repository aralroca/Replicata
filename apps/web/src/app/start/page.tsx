import type { Metadata } from "next";

import { Onboarding } from "@/components/onboarding";

export const metadata: Metadata = { title: "Start in 60 seconds" };

export default function StartPage() {
  return (
    <div className="grid gap-6">
      <header>
        <h1 className="text-2xl font-bold">Start in 60 seconds</h1>
        <p className="mt-2 text-muted">Three steps from zero to publishing or replicating an experiment.</p>
      </header>
      <Onboarding />
    </div>
  );
}

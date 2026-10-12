"use client";

import { StepEconomics } from "@/components/publish/step-economics";
import { StepReview } from "@/components/publish/step-review";
import { StepSpec } from "@/components/publish/step-spec";
import { LoadFailed, Loading } from "@/components/ui";
import { useConfig } from "@/hooks/use-coordinator";
import { usePublishWizard } from "@/hooks/use-publish-wizard";
import { toEconomics } from "@/lib/publish/economics-form";
import { toSpec } from "@/lib/publish/spec-form";
import type { SpecIssue } from "@/lib/spec/validate";
import type { CoordinatorConfig } from "@/lib/types";

type Wizard = ReturnType<typeof usePublishWizard>;

function IssueSummary({ issues }: { issues: SpecIssue[] }) {
  return issues.length === 0 ? null : (
    <div role="alert" className="rounded-lg border border-danger/60 bg-danger/10 p-4 text-sm">
      <p className="font-semibold text-danger">Fix these before continuing:</p>
      <ul className="mt-1 list-disc pl-5" data-testid="issues">
        {issues.map((issue) => <li key={`${issue.field}:${issue.message}`}>{issue.message}</li>)}
      </ul>
    </div>
  );
}

function CurrentStep({ wizard, config }: { wizard: Wizard; config: CoordinatorConfig }) {
  const steps = {
    1: () => <StepSpec form={wizard.spec} setForm={wizard.setSpec} issues={wizard.issues} />,
    2: () => <StepEconomics form={wizard.economics} setForm={wizard.setEconomics} issues={wizard.issues} />,
    3: () => <StepReview spec={toSpec(wizard.spec)} economics={toEconomics(wizard.economics, config.decimals)} config={config} />,
  };

  return steps[wizard.step]();
}

function Navigation({ wizard }: { wizard: Wizard }) {
  return (
    <div className="flex gap-3">
      {wizard.step > 1 && <button type="button" className="btn btn-ghost" onClick={wizard.back}>Back</button>}
      {wizard.step < 3 && <button type="button" className="btn btn-primary" onClick={wizard.next}>Next</button>}
    </div>
  );
}

function WizardForm({ config }: { config: CoordinatorConfig }) {
  const wizard = usePublishWizard(config.decimals);

  return (
    <div className="card grid gap-6">
      <p className="text-sm text-muted" aria-live="polite">Step {wizard.step} of 3</p>
      <CurrentStep wizard={wizard} config={config} />
      <IssueSummary issues={wizard.issues} />
      <Navigation wizard={wizard} />
    </div>
  );
}

export function PublishWizard() {
  const { data: config, error } = useConfig();

  if (error) return <LoadFailed what="the coordinator config" error={error} />;

  return config ? <WizardForm config={config} /> : <Loading />;
}

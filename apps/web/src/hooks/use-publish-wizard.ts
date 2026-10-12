"use client";

import { useState } from "react";

import { DEFAULT_ECONOMICS_FORM, economicsIssues, type EconomicsForm } from "@/lib/publish/economics-form";
import { EMPTY_SPEC_FORM, specFormIssues, type SpecForm } from "@/lib/publish/spec-form";
import type { SpecIssue } from "@/lib/spec/validate";

export type Step = 1 | 2 | 3;

/** Spec issue field (`image_digest`, `inputs.0.url`, `resources`…) → form field name. */
export function formKey(field: string): string {
  const aliases: Record<string, string> = { image_digest: "imageDigest", resources: "cpus" };
  const root = field.split(".")[0];

  return aliases[root] ?? root;
}

export function errorsFor(issues: SpecIssue[], key: string): string[] {
  return issues.filter((issue) => formKey(issue.field) === key).map((issue) => issue.message);
}

function issuesFor(step: Step, spec: SpecForm, economics: EconomicsForm, decimals: number) {
  const byStep: Record<Step, () => SpecIssue[]> = {
    1: () => specFormIssues(spec),
    2: () => economicsIssues(economics, decimals),
    3: () => [],
  };

  return byStep[step]();
}

function useForms() {
  const [spec, setSpec] = useState<SpecForm>(EMPTY_SPEC_FORM);
  const [economics, setEconomics] = useState<EconomicsForm>(DEFAULT_ECONOMICS_FORM);

  return { spec, setSpec, economics, setEconomics };
}

const nextStep = (step: Step) => Math.min(step + 1, 3) as Step;
const previousStep = (step: Step) => Math.max(step - 1, 1) as Step;

/** Step navigation: "Next" only advances when the current step has no blocking issue. */
function useSteps(blocking: (step: Step) => SpecIssue[]) {
  const [step, setStep] = useState<Step>(1);
  const [touched, setTouched] = useState(false);
  const next = () => {
    const ok = blocking(step).length === 0;

    setTouched(!ok);
    setStep(ok ? nextStep(step) : step);
  };

  return { step, touched, next, back: () => setStep(previousStep) };
}

export function usePublishWizard(decimals = 6) {
  const forms = useForms();
  const blocking = (step: Step) => issuesFor(step, forms.spec, forms.economics, decimals);
  const steps = useSteps(blocking);

  return { ...forms, ...steps, issues: steps.touched ? blocking(steps.step) : [] };
}

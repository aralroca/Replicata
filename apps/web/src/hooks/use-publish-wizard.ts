"use client";

import { useState } from "react";

import {
  DEFAULT_ECONOMICS_FORM,
  economicsIssues,
  type EconomicsForm,
} from "@/lib/publish/economics-form";
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

export function usePublishWizard(decimals = 6) {
  const [step, setStep] = useState<Step>(1);
  const [spec, setSpec] = useState<SpecForm>(EMPTY_SPEC_FORM);
  const [economics, setEconomics] = useState<EconomicsForm>(DEFAULT_ECONOMICS_FORM);
  const [touched, setTouched] = useState(false);
  const issues = touched ? issuesFor(step, spec, economics, decimals) : [];
  const next = () => {
    const blocking = issuesFor(step, spec, economics, decimals);

    setTouched(blocking.length > 0);
    if (blocking.length === 0) setStep((current) => Math.min(current + 1, 3) as Step);
  };

  return { step, spec, setSpec, economics, setEconomics, issues, next, back: () => setStep((s) => Math.max(s - 1, 1) as Step) };
}

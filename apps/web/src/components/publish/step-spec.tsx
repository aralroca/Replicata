import { binder } from "@/components/publish/bind";
import { FieldList, type FieldSpec } from "@/components/publish/field-list";
import type { SpecForm } from "@/lib/publish/spec-form";
import type { SpecIssue } from "@/lib/spec/validate";

type Props = { form: SpecForm; setForm: (form: SpecForm) => void; issues: SpecIssue[] };

const MAIN_FIELDS: FieldSpec<SpecForm>[] = [
  { key: "title", label: "Title", placeholder: "Monte-Carlo π" },
  { key: "description", label: "Description", multiline: true },
  { key: "image", label: "Image repository", placeholder: "ghcr.io/you/experiment", hint: "No tag: pulled by digest." },
  {
    key: "imageDigest",
    label: "Image digest",
    placeholder: "sha256:…",
    hint: "docker buildx imagetools inspect → Digest",
  },
  {
    key: "cmd",
    label: "Command",
    multiline: true,
    placeholder: "python\n/app/main.py",
    hint: "One argument per line.",
  },
  { key: "env", label: "Environment", multiline: true, placeholder: "PYTHONHASHSEED=0", hint: "KEY=VALUE per line." },
  {
    key: "inputs",
    label: "Inputs",
    multiline: true,
    placeholder: "<url> <sha256> <path>",
    hint: "One input per line.",
  },
];

const RESOURCE_FIELDS: FieldSpec<SpecForm>[] = [
  { key: "seed", label: "Seed", inputMode: "numeric" },
  { key: "cpus", label: "CPUs", inputMode: "numeric", hint: "1–16" },
  { key: "memMb", label: "Memory (MB)", inputMode: "numeric", hint: "64–16384" },
  { key: "timeoutS", label: "Timeout (s)", inputMode: "numeric", hint: "1–3600" },
];

export function StepSpec({ form, setForm, issues }: Props) {
  const bind = binder(form, setForm, issues);

  return (
    <fieldset className="grid gap-4">
      <legend className="mb-2 text-lg font-semibold">1 · What to run</legend>
      <FieldList fields={MAIN_FIELDS} bind={bind} className="grid gap-4" />
      <FieldList fields={RESOURCE_FIELDS} bind={bind} className="grid gap-4 sm:grid-cols-4" />
    </fieldset>
  );
}

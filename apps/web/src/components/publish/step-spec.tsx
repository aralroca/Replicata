import { binder } from "@/components/publish/bind";
import { Field } from "@/components/publish/field";
import type { SpecForm } from "@/lib/publish/spec-form";
import type { SpecIssue } from "@/lib/spec/validate";

type Props = { form: SpecForm; setForm: (form: SpecForm) => void; issues: SpecIssue[] };

type Bind = ReturnType<typeof binder<SpecForm>>;

function WhatFields({ bind }: { bind: Bind }) {
  return (
    <>
      <Field label="Title" placeholder="Monte-Carlo π" {...bind("title")} />
      <Field label="Description" multiline {...bind("description")} />
    </>
  );
}

function ImageFields({ bind }: { bind: Bind }) {
  return (
    <>
      <Field label="Image repository" placeholder="ghcr.io/you/experiment" hint="No tag: the image is pulled by digest." {...bind("image")} />
      <Field label="Image digest" placeholder="sha256:…" hint="docker buildx imagetools inspect <image> → Digest" {...bind("imageDigest")} />
      <Field label="Command" multiline placeholder={"python\n/app/main.py"} hint="One argument per line." {...bind("cmd")} />
    </>
  );
}

function DataFields({ bind }: { bind: Bind }) {
  return (
    <>
      <Field label="Environment" multiline placeholder="PYTHONHASHSEED=0" hint="KEY=VALUE per line." {...bind("env")} />
      <Field label="Inputs" multiline placeholder="https://… <sha256> data/file.csv" hint="<url> <sha256> <path> per line." {...bind("inputs")} />
    </>
  );
}

function ResourceFields({ bind }: { bind: Bind }) {
  return (
    <div className="grid gap-4 sm:grid-cols-4">
      <Field label="Seed" inputMode="numeric" {...bind("seed")} />
      <Field label="CPUs" inputMode="numeric" hint="1–16" {...bind("cpus")} />
      <Field label="Memory (MB)" inputMode="numeric" hint="64–16384" {...bind("memMb")} />
      <Field label="Timeout (s)" inputMode="numeric" hint="1–3600" {...bind("timeoutS")} />
    </div>
  );
}

export function StepSpec({ form, setForm, issues }: Props) {
  const bind = binder(form, setForm, issues);

  return (
    <fieldset className="grid gap-4">
      <legend className="mb-2 text-lg font-semibold">1 · What to run</legend>
      <WhatFields bind={bind} />
      <ImageFields bind={bind} />
      <DataFields bind={bind} />
      <ResourceFields bind={bind} />
    </fieldset>
  );
}

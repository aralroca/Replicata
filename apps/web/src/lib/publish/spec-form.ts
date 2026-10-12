// Step 1 of the publish wizard: raw form strings ⇄ JobSpec, plus validation issues.
import { SPEC_VERSION, validateSpec, type SpecIssue } from "@/lib/spec/validate";
import type { JobSpec, SpecInput } from "@/lib/types";

export type SpecForm = {
  title: string;
  description: string;
  image: string;
  imageDigest: string;
  /** One argument per line. */
  cmd: string;
  /** `KEY=VALUE` per line. */
  env: string;
  /** `<url> <sha256> <path>` per line. */
  inputs: string;
  seed: string;
  cpus: string;
  memMb: string;
  timeoutS: string;
};

export const EMPTY_SPEC_FORM: SpecForm = {
  title: "",
  description: "",
  image: "",
  imageDigest: "",
  cmd: "",
  env: "",
  inputs: "",
  seed: "42",
  cpus: "1",
  memMb: "512",
  timeoutS: "300",
};

const U64_MAX = 2n ** 64n - 1n;

const lines = (text: string) =>
  text
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean);

const toInt = (text: string) => (/^\d+$/.test(text.trim()) ? Number(text.trim()) : NaN);

export function parseSeed(text: string): bigint | null {
  const seed = /^\d+$/.test(text.trim()) ? BigInt(text.trim()) : null;

  return seed !== null && seed <= U64_MAX ? seed : null;
}

export function parseEnv(text: string): Record<string, string> | null {
  const pairs = lines(text).map((line) => /^([^=\s]+)=(.*)$/.exec(line));

  return pairs.every(Boolean) ? Object.fromEntries(pairs.map((m) => [m![1], m![2]])) : null;
}

export function parseInputs(text: string): SpecInput[] | null {
  const parts = lines(text).map((line) => line.split(/\s+/));

  return parts.every((p) => p.length === 3)
    ? parts.map(([url, sha256, path]) => ({ url, sha256, path }))
    : null;
}

const textFields = (form: SpecForm) => ({
  version: SPEC_VERSION,
  title: form.title,
  description: form.description,
  image: form.image.trim(),
  image_digest: form.imageDigest.trim(),
});

const listFields = (form: SpecForm) => ({
  cmd: form.cmd.split("\n").filter((arg) => arg !== ""),
  env: parseEnv(form.env) ?? {},
  inputs: parseInputs(form.inputs) ?? [],
});

const numberFields = (form: SpecForm) => ({
  cpus: toInt(form.cpus),
  mem_mb: toInt(form.memMb),
  timeout_s: toInt(form.timeoutS),
  seed: parseSeed(form.seed) ?? 0n,
});

export function toSpec(form: SpecForm): JobSpec {
  return { ...textFields(form), ...listFields(form), ...numberFields(form) };
}

const FORMAT_ISSUES: { field: string; message: string; ok: (form: SpecForm) => boolean }[] = [
  { field: "env", message: "env: one KEY=VALUE per line", ok: (f) => parseEnv(f.env) !== null },
  {
    field: "inputs",
    message: "inputs: one `<url> <sha256> <path>` per line",
    ok: (f) => parseInputs(f.inputs) !== null,
  },
  { field: "seed", message: "seed must be an integer in 0..=2^64-1", ok: (f) => parseSeed(f.seed) !== null },
];

/** Form-format problems first, then the exact `JobSpec::validate` checks. */
export function specFormIssues(form: SpecForm): SpecIssue[] {
  const format = FORMAT_ISSUES.filter((rule) => !rule.ok(form));

  return [...format.map(({ field, message }) => ({ field, message })), ...validateSpec(toSpec(form))];
}

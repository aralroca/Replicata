// Mirror of `JobSpec::validate` (crates/core/src/spec.rs): same checks, same order, same messages.
import { isHex32 } from "@/lib/hex";
import type { JobSpec, SpecInput } from "@/lib/types";

export const SPEC_VERSION = 1;
export const MAX_TIMEOUT_S = 3_600;
export const MAX_MEM_MB = 16_384;
export const MAX_CPUS = 16;

export type SpecIssue = { field: string; message: string };

type Rule<T> = { field: string; message: (value: T) => string; ok: (value: T) => boolean };

const inRange = (value: number, min: number, max: number) =>
  Number.isInteger(value) && value >= min && value <= max;

export function isSafeRelativePath(path: string): boolean {
  const segmentsOk = path.split("/").every((seg) => seg !== "" && seg !== "." && seg !== "..");

  return path !== "" && !path.startsWith("/") && !path.includes("\\") && segmentsOk;
}

const SPEC_RULES: Rule<JobSpec>[] = [
  {
    field: "version",
    message: (spec) => `unsupported spec version ${spec.version}`,
    ok: (spec) => spec.version === SPEC_VERSION,
  },
  {
    field: "title",
    message: () => "title must be 1..=120 characters",
    ok: (spec) => spec.title.trim() !== "" && Array.from(spec.title).length <= 120,
  },
  {
    field: "image",
    message: () => "image must be a non-empty repository name without tag or digest",
    ok: (spec) => spec.image !== "" && !spec.image.includes("@") && !/\s/u.test(spec.image),
  },
  {
    field: "image_digest",
    message: () => "image_digest must look like sha256:<64 hex chars>",
    ok: (spec) => /^sha256:/.test(spec.image_digest) && isHex32(spec.image_digest.slice(7)),
  },
  { field: "cmd", message: () => "cmd must not be empty", ok: (spec) => spec.cmd.length > 0 },
];

const INPUT_RULES: Rule<[SpecInput, number]>[] = [
  {
    field: "sha256",
    message: ([, i]) => `input ${i}: sha256 must be 64 hex chars`,
    ok: ([input]) => isHex32(input.sha256),
  },
  {
    field: "path",
    message: ([, i]) => `input ${i}: path must be relative, non-empty and without \`..\``,
    ok: ([input]) => isSafeRelativePath(input.path),
  },
  {
    field: "url",
    message: ([, i]) => `input ${i}: url must be http(s)`,
    ok: ([input]) => /^https?:\/\//.test(input.url),
  },
];

const RESOURCES_RULE: Rule<JobSpec> = {
  field: "resources",
  message: () =>
    `resources out of range (cpus 1..=${MAX_CPUS}, mem_mb 64..=${MAX_MEM_MB}, timeout_s 1..=${MAX_TIMEOUT_S})`,
  ok: (spec) =>
    inRange(spec.cpus, 1, MAX_CPUS) &&
    inRange(spec.mem_mb, 64, MAX_MEM_MB) &&
    inRange(spec.timeout_s, 1, MAX_TIMEOUT_S),
};

function check<T>(rules: Rule<T>[], value: T, prefix = ""): SpecIssue[] {
  return rules
    .filter((rule) => !rule.ok(value))
    .map((rule) => ({ field: `${prefix}${rule.field}`, message: rule.message(value) }));
}

/** Every failing check, in the order Rust evaluates them (the first one is Rust's error). */
export function validateSpec(spec: JobSpec): SpecIssue[] {
  const inputIssues = spec.inputs.flatMap((input, i) =>
    check(INPUT_RULES, [input, i] as [SpecInput, number], `inputs.${i}.`)
  );

  return [...check(SPEC_RULES, spec), ...inputIssues, ...check([RESOURCES_RULE], spec)];
}

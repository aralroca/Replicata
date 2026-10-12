import { expect, test } from "vitest";

import { isSafeRelativePath, validateSpec } from "@/lib/spec/validate";
import type { JobSpec } from "@/lib/types";

// Same fixture and cases as `crates/core/src/spec.rs` `validation_errors`.
const SAMPLE: JobSpec = {
  version: 1,
  title: "Monte-Carlo π",
  description: "Estimate π with 10M samples",
  image: "ghcr.io/aralroca/replicata-pi",
  image_digest: `sha256:${"ab".repeat(32)}`,
  cmd: ["python", "/app/main.py"],
  env: { OMP_NUM_THREADS: "1" },
  inputs: [],
  cpus: 1,
  mem_mb: 512,
  timeout_s: 120,
  seed: 42n,
};

const sample = (): JobSpec => ({ ...SAMPLE });

const input = (url: string, sha256: string, path: string) => [{ url, sha256, path }];

const CASES: [string, Partial<JobSpec>, string][] = [
  ["version", { version: 2 }, "unsupported spec version 2"],
  ["title", { title: " " }, "title must be 1..=120 characters"],
  ["long title", { title: "x".repeat(121) }, "title must be 1..=120 characters"],
  ["digest", { image_digest: "sha256:xyz" }, "image_digest must look like sha256:<64 hex chars>"],
  ["image", { image: "alpine@sha256" }, "image must be a non-empty repository name without tag or digest"],
  ["cmd", { cmd: [] }, "cmd must not be empty"],
  ["timeout", { timeout_s: 0 }, "resources out of range (cpus 1..=16, mem_mb 64..=16384, timeout_s 1..=3600)"],
  ["mem", { mem_mb: 1 }, "resources out of range (cpus 1..=16, mem_mb 64..=16384, timeout_s 1..=3600)"],
  [
    "path",
    { inputs: input("https://x", "00".repeat(32), "../etc") },
    "input 0: path must be relative, non-empty and without `..`",
  ],
  ["url", { inputs: input("ftp://x", "00".repeat(32), "a") }, "input 0: url must be http(s)"],
  ["hash", { inputs: input("https://x", "zz", "a") }, "input 0: sha256 must be 64 hex chars"],
];

test("the sample spec is valid", () => {
  expect(validateSpec(sample())).toEqual([]);
});

test.each(CASES)("rejects bad %s like Rust", (_name, patch, message) => {
  expect(validateSpec({ ...sample(), ...patch })[0]?.message).toBe(message);
});

test("120 code points are fine even if they are multi-unit in UTF-16", () => {
  expect(validateSpec({ ...sample(), title: "😀".repeat(120) })).toEqual([]);
});

test("uppercase hex is accepted like the Rust `hex` crate", () => {
  expect(validateSpec({ ...sample(), image_digest: `sha256:${"AB".repeat(32)}` })).toEqual([]);
});

test.each([
  ["data/a.csv", true],
  ["a", true],
  ["", false],
  ["/abs", false],
  ["a//b", false],
  ["a/./b", false],
  ["a\\b", false],
  ["..", false],
])("isSafeRelativePath(%j) = %s", (path, ok) => {
  expect(isSafeRelativePath(path)).toBe(ok);
});

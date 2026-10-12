import { expect, test } from "vitest";

import { formKey } from "@/hooks/use-publish-wizard";
import { DEFAULT_ECONOMICS_FORM, economicsIssues, toEconomics } from "@/lib/publish/economics-form";
import {
  EMPTY_SPEC_FORM,
  parseEnv,
  parseInputs,
  parseSeed,
  specFormIssues,
  toSpec,
  type SpecForm,
} from "@/lib/publish/spec-form";

const valid: SpecForm = {
  ...EMPTY_SPEC_FORM,
  title: "Monte-Carlo π",
  image: "ghcr.io/aralroca/replicata-pi",
  imageDigest: `sha256:${"0f".repeat(32)}`,
  cmd: "python\n/app/main.py",
  env: "PYTHONHASHSEED=0\nOMP_NUM_THREADS=1",
  inputs: `https://example.org/data.csv ${"aa".repeat(32)} data/data.csv`,
};

test("a filled form becomes the JobSpec the worker expects", () => {
  expect(specFormIssues(valid)).toEqual([]);
  expect(toSpec(valid)).toMatchObject({
    version: 1,
    cmd: ["python", "/app/main.py"],
    env: { OMP_NUM_THREADS: "1", PYTHONHASHSEED: "0" },
    inputs: [{ url: "https://example.org/data.csv", sha256: "aa".repeat(32), path: "data/data.csv" }],
    seed: 42n,
    cpus: 1,
  });
});

test("the empty form reports Rust's errors", () => {
  const messages = specFormIssues(EMPTY_SPEC_FORM).map((issue) => issue.message);

  expect(messages).toEqual([
    "title must be 1..=120 characters",
    "image must be a non-empty repository name without tag or digest",
    "image_digest must look like sha256:<64 hex chars>",
    "cmd must not be empty",
  ]);
});

test("format problems are reported before spec checks", () => {
  const issues = specFormIssues({ ...valid, env: "NOPE", seed: "-1", cpus: "x" });

  expect(issues.map((issue) => issue.field)).toEqual(["env", "seed", "resources"]);
});

test("parsers", () => {
  expect(parseSeed("18446744073709551615")).toBe(18446744073709551615n);
  expect(parseSeed("18446744073709551616")).toBeNull();
  expect(parseEnv("A=1\nB=x=y\n\n")).toEqual({ A: "1", B: "x=y" });
  expect(parseEnv("=1")).toBeNull();
  expect(parseInputs("only-two parts")).toBeNull();
});

const DEFAULT_ECONOMICS = {
  reward: 3_000_000n,
  bond: 1_000_000n,
  accessPrice: 10_000n,
  k: 3,
  maxClaims: 3,
  commitSecs: 1800,
  revealSecs: 1800,
  authorShareBps: 7000,
};

test("default economics are valid and convert to base units", () => {
  expect(economicsIssues(DEFAULT_ECONOMICS_FORM)).toEqual([]);
  expect(toEconomics(DEFAULT_ECONOMICS_FORM)).toEqual(DEFAULT_ECONOMICS);
});

test("economics mirror the program checks", () => {
  const form = {
    ...DEFAULT_ECONOMICS_FORM,
    reward: "0",
    k: "4",
    maxClaims: "6",
    authorSharePct: "101",
    revealMinutes: "0",
  };
  const fields = economicsIssues(form).map((issue) => issue.field);

  expect(fields).toEqual(["reward", "k", "commitMinutes", "authorSharePct"]);
});

test("spec issue fields map to form inputs", () => {
  expect(formKey("image_digest")).toBe("imageDigest");
  expect(formKey("inputs.0.url")).toBe("inputs");
  expect(formKey("resources")).toBe("cpus");
  expect(formKey("title")).toBe("title");
});

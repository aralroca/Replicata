import { readFileSync } from "node:fs";

import { expect, test } from "vitest";

import { canonicalJson, canonicalSpecJson, compareUtf8, specHashHex } from "@/lib/spec/canonical";
import type { JobSpec } from "@/lib/types";

type Vector = { canonical: string; spec: JobSpec; spec_hash: string };

const VECTORS_PATH = new URL("../../../../../vectors/core.json", import.meta.url);

/** JSON.parse that keeps integers beyond 2^53 exact (u64 seeds) via the reviver `source`. */
function parseExact(text: string): { specs: Vector[] } {
  const reviver = (key: string, value: unknown, context?: { source?: string }) =>
    key === "seed" && context?.source ? BigInt(context.source) : value;

  return JSON.parse(text, reviver as Parameters<typeof JSON.parse>[1]);
}

const vectors = parseExact(readFileSync(VECTORS_PATH, "utf8")).specs;

test("there are spec vectors to check", () => {
  expect(vectors.length).toBeGreaterThanOrEqual(2);
});

test.each(vectors.map((v) => [v.spec.title, v] as const))("canonical JSON matches Rust for %s", (_title, vector) => {
  expect(canonicalSpecJson(vector.spec)).toBe(vector.canonical);
});

test.each(vectors.map((v) => [v.spec.title, v] as const))("spec_hash matches Rust for %s", async (_title, vector) => {
  expect(await specHashHex(vector.spec)).toBe(vector.spec_hash);
});

test("u64 seed above 2^53 is serialised exactly", () => {
  expect(canonicalJson({ seed: 18446744073709551615n })).toBe('{"seed":18446744073709551615}');
});

test("keys sort bytewise (UTF-8), not by UTF-16 code unit", () => {
  // U+FF5E (3 UTF-8 bytes, EF BD 9E) sorts before U+1F600 (F0 9F 98 80) bytewise,
  // although its UTF-16 unit 0xFF5E is greater than the surrogate 0xD83D.
  expect(["😀", "～", "a"].sort(compareUtf8)).toEqual(["a", "～", "😀"]);
  expect(canonicalJson({ "😀": 1, "～": 2 })).toBe('{"～":2,"😀":1}');
});

test("key order of the input does not matter", () => {
  expect(canonicalJson({ b: 1, a: { d: [1, "x"], c: null } })).toBe('{"a":{"c":null,"d":[1,"x"]},"b":1}');
});

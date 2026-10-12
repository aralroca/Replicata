import { expect, test } from "vitest";

import { PHASE_META, STATUS_META, statusSummary, timeline } from "@/lib/status";
import type { ExperimentView } from "@/lib/types";

type Counts = Parameters<typeof timeline>[0];

const base: Counts = {
  k: 3,
  max_claims: 3,
  claims_count: 0,
  commits_count: 0,
  reveals_count: 0,
  status: "open",
  phase: "claiming",
};

const states = (view: Partial<Counts>) => timeline({ ...base, ...view }).map((step) => step.state);

test("every status and phase has a label", () => {
  expect(Object.keys(STATUS_META).sort()).toEqual(["cancelled", "open", "replicated", "unresolved"]);
  expect(Object.keys(PHASE_META).sort()).toEqual(["claiming", "finalizable", "revealing", "settled"]);
});

test.each<[string, Partial<Counts>, string[]]>([
  ["fresh", {}, ["active", "pending", "pending", "pending"]],
  ["claimed", { claims_count: 3, commits_count: 1 }, ["done", "active", "pending", "pending"]],
  [
    "revealing",
    { claims_count: 3, commits_count: 3, reveals_count: 1, phase: "revealing" },
    ["done", "done", "active", "pending"],
  ],
  [
    "finalizable",
    { claims_count: 3, commits_count: 3, reveals_count: 3, phase: "finalizable" },
    ["done", "done", "done", "active"],
  ],
  [
    "replicated",
    { claims_count: 3, commits_count: 3, reveals_count: 3, status: "replicated", phase: "settled" },
    ["done", "done", "done", "done"],
  ],
  [
    "unresolved",
    { claims_count: 2, commits_count: 1, status: "unresolved", phase: "settled" },
    ["pending", "pending", "pending", "failed"],
  ],
  ["cancelled", { status: "cancelled", phase: "settled" }, ["pending", "pending", "pending", "failed"]],
])("timeline: %s", (_name, view, expected) => {
  expect(states(view)).toEqual(expected);
});

test("timeline details show progress against k", () => {
  expect(
    timeline({ ...base, claims_count: 2 })
      .map((s) => s.detail)
      .slice(0, 3)
  ).toEqual(["2/3", "0/3", "0/3"]);
});

test.each<[Partial<ExperimentView>, string]>([
  [{ status: "replicated", agree_count: 3 }, "Replicated ✅ 3/3"],
  [{ status: "open", commits_count: 1 }, "Replicating 1/3"],
  [{ status: "unresolved" }, "Unresolved"],
  [{ status: "cancelled" }, "Cancelled"],
])("statusSummary %o", (view, text) => {
  expect(statusSummary({ status: "open", agree_count: 0, k: 3, commits_count: 0, ...view })).toBe(text);
});

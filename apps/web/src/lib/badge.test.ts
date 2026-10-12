import { expect, test } from "vitest";

import { experimentBadge, renderBadge } from "@/lib/badge";

const view = { k: 3, agree_count: 0, commits_count: 0 };

test("replicated badge", () => {
  expect(experimentBadge({ ...view, status: "replicated", agree_count: 3 })).toMatchSnapshot();
});

test("replicating badge", () => {
  expect(experimentBadge({ ...view, status: "open", commits_count: 1 })).toMatchSnapshot();
});

test("unresolved, cancelled and unknown badges", () => {
  expect(experimentBadge({ ...view, status: "unresolved" })).toMatchSnapshot();
  expect(experimentBadge({ ...view, status: "cancelled" })).toMatchSnapshot();
  expect(experimentBadge(null)).toMatchSnapshot();
});

test("badge text is XML-escaped", () => {
  expect(renderBadge(`<a&"b>`, "#000")).toContain("&lt;a&amp;&quot;b&gt;");
});

test("badge is valid-looking SVG with an accessible label", () => {
  const svg = experimentBadge({ ...view, status: "replicated", agree_count: 3 });

  expect(svg).toMatch(/^<svg xmlns="http:\/\/www\.w3\.org\/2000\/svg"/);
  expect(svg).toContain('aria-label="replicata: Replicated ✅ 3/3"');
});

import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { Footer } from "./footer";

const GITLAB_URL = "https://gitlab.com/gitlab-ai-hackathon/transcend-october-2026/5767991/showcase";

describe("Footer", () => {
  it("links the GitLab project", () => {
    const html = renderToStaticMarkup(createElement(Footer));
    const match = html.match(/<a [^>]*href="([^"]+)"[^>]*>GitLab<\/a>/);

    expect(match?.[1]).toBe(GITLAB_URL);
  });
});

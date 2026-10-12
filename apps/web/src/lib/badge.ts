// Shields-style flat badge: "replicata | Replicated ✅ 3/3".
import { statusSummary } from "@/lib/status";
import type { ExperimentStatus, ExperimentView } from "@/lib/types";

const COLORS: Record<ExperimentStatus | "unknown", string> = {
  replicated: "#2ea44f",
  open: "#0969da",
  unresolved: "#d29922",
  cancelled: "#6e7781",
  unknown: "#6e7781",
};

const LABEL = "replicata";

/** Rough Verdana 11px advance; good enough for a badge, deterministic for snapshots. */
function textWidth(text: string): number {
  return Array.from(text).reduce((width, char) => width + (/[\u{1F000}-\u{1FFFF}✅]/u.test(char) ? 12 : 7), 10);
}

function escapeXml(text: string): string {
  const entities: Record<string, string> = {
    "&": "&amp;",
    "<": "&lt;",
    ">": "&gt;",
    '"': "&quot;",
  };

  return text.replace(/[&<>"]/g, (char) => entities[char]);
}

export function renderBadge(message: string, color: string): string {
  const left = textWidth(LABEL);
  const right = textWidth(message);
  const width = left + right;
  const text = (x: number, value: string) =>
    `<text x="${x}" y="14" fill="#fff" text-anchor="middle">${escapeXml(value)}</text>`;

  return `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="20" role="img" aria-label="${escapeXml(`${LABEL}: ${message}`)}"><title>${escapeXml(`${LABEL}: ${message}`)}</title><rect width="${left}" height="20" fill="#555"/><rect x="${left}" width="${right}" height="20" fill="${color}"/><g font-family="Verdana,DejaVu Sans,sans-serif" font-size="11">${text(left / 2, LABEL)}${text(left + right / 2, message)}</g></svg>`;
}

export function experimentBadge(
  view: Pick<ExperimentView, "status" | "agree_count" | "k" | "commits_count"> | null
): string {
  return view ? renderBadge(statusSummary(view), COLORS[view.status]) : renderBadge("not found", COLORS.unknown);
}

// GET /e/:id/badge.svg — shields-style replication badge for papers and READMEs.
import { ApiError, getJson } from "@/lib/api";
import { experimentBadge, renderBadge } from "@/lib/badge";
import { serverCoordinatorUrl } from "@/lib/env";
import type { ExperimentView } from "@/lib/types";

export const dynamic = "force-dynamic";

// Always 200: image embeds (GitHub camo, preprint servers) hide non-2xx images.
function svg(body: string): Response {
  return new Response(body, {
    headers: {
      "Content-Type": "image/svg+xml; charset=utf-8",
      "Cache-Control": "public, max-age=30",
    },
  });
}

async function fetchExperiment(id: string): Promise<ExperimentView | null> {
  return getJson<ExperimentView>(`/v1/experiments/${encodeURIComponent(id)}`, serverCoordinatorUrl()).catch(
    (error: unknown) => (error instanceof ApiError && error.status === 404 ? null : Promise.reject(error))
  );
}

export async function GET(_request: Request, { params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;

  return fetchExperiment(id)
    .then((experiment) => svg(experimentBadge(experiment)))
    .catch(() => svg(renderBadge("unavailable", "#6e7781")));
}

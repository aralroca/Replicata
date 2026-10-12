// Thin typed client for the coordinator's free endpoints (docs/API.md).
import { COORDINATOR_URL } from "@/lib/env";
import { canonicalSpecJson } from "@/lib/spec/canonical";
import type { AuthorView, CoordinatorConfig, ExperimentView, JobSpec, WorkerView } from "@/lib/types";

export class ApiError extends Error {
  constructor(
    readonly status: number,
    message: string
  ) {
    super(message);
  }
}

async function errorMessage(response: Response): Promise<string> {
  const body = await response.json().catch(() => null);

  return body?.message ?? body?.error ?? `${response.status} ${response.statusText}`;
}

export async function request<T>(path: string, init?: RequestInit, base = COORDINATOR_URL) {
  const response = await fetch(`${base}${path}`, { cache: "no-store", ...init });

  if (!response.ok) {
    throw new ApiError(response.status, await errorMessage(response));
  }

  return response.json() as Promise<T>;
}

export function getJson<T>(path: string, base = COORDINATOR_URL): Promise<T> {
  return request<T>(path, undefined, base);
}

export const api = {
  config: () => getJson<CoordinatorConfig>("/v1/config"),
  experiments: (query = "") => getJson<ExperimentView[]>(`/v1/experiments${query}`),
  experiment: (id: string) => getJson<ExperimentView>(`/v1/experiments/${id}`),
  worker: (pubkey: string) => getJson<WorkerView>(`/v1/workers/${pubkey}`),
  author: (pubkey: string) => getJson<AuthorView>(`/v1/authors/${pubkey}`),
};

/** POST /v1/specs with the canonical bytes (u64 seed stays exact). Returns the spec hash. */
export async function postSpec(spec: JobSpec): Promise<string> {
  const init = {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: canonicalSpecJson(spec),
  };
  const { spec_hash } = await request<{ spec_hash: string }>("/v1/specs", init);

  return spec_hash;
}

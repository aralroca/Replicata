"use client";

import useSWR from "swr";

import { api } from "@/lib/api";

const LIVE = { refreshInterval: 3_000 };

export function useConfig() {
  return useSWR("config", api.config, { revalidateOnFocus: false });
}

export function useExperiments(query = "") {
  return useSWR(["experiments", query], () => api.experiments(query), { refreshInterval: 5_000 });
}

/** 3 s while the timeline can move; settled experiments only change on royalties. */
function experimentRefresh(experiment?: { phase: string }) {
  return experiment?.phase === "settled" ? 30_000 : 3_000;
}

/** Polls the experiment so the timeline moves on its own. */
export function useExperiment(id: string) {
  return useSWR(["experiment", id], () => api.experiment(id), {
    refreshInterval: experimentRefresh,
  });
}

export function useWorker(pubkey: string) {
  return useSWR(["worker", pubkey], () => api.worker(pubkey), LIVE);
}

export function useAuthor(pubkey: string) {
  return useSWR(["author", pubkey], () => api.author(pubkey), LIVE);
}

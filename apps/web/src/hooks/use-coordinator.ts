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

/** Polls the experiment every 3 s so the timeline moves on its own. */
export function useExperiment(id: string) {
  return useSWR(["experiment", id], () => api.experiment(id), LIVE);
}

export function useWorker(pubkey: string) {
  return useSWR(["worker", pubkey], () => api.worker(pubkey), LIVE);
}

export function useAuthor(pubkey: string) {
  return useSWR(["author", pubkey], () => api.author(pubkey), LIVE);
}

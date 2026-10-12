"use client";

import { useWalletUiSigner, type UiWalletAccount } from "@wallet-ui/react";
import { useRouter } from "next/navigation";
import { useState } from "react";

import type { UiError } from "@/lib/errors";
import { publishError, publishExperiment, type PublishInput, type PublishStage } from "@/lib/publish/publish";
import type { Economics } from "@/lib/tx/create-experiment";
import type { CoordinatorConfig, JobSpec } from "@/lib/types";

type State = { stage: PublishStage | "idle"; error: UiError | null };

function runPublish(input: PublishInput, done: (experiment: string) => void, fail: (error: unknown) => void) {
  input.onStage("uploading");
  publishExperiment(input)
    .then(({ experiment }) => done(experiment))
    .catch(fail);
}

export function usePublish(account: UiWalletAccount, config: CoordinatorConfig) {
  const signer = useWalletUiSigner({ account });
  const router = useRouter();
  const [state, setState] = useState<State>({ stage: "idle", error: null });
  const onStage = (stage: PublishStage) => setState({ stage, error: null });
  const fail = (error: unknown) => setState({ stage: "idle", error: publishError(error, config) });
  const publish = (spec: JobSpec, economics: Economics) =>
    runPublish({ spec, economics, signer, config, onStage }, (pda) => router.push(`/e/${pda}`), fail);

  return { ...state, publish };
}

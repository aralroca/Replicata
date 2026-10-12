"use client";

import { useWalletUiSigner, type UiWalletAccount } from "@wallet-ui/react";
import { useRouter } from "next/navigation";
import { useState } from "react";

import type { UiError } from "@/lib/errors";
import { publishError, publishExperiment, type PublishStage } from "@/lib/publish/publish";
import type { Economics } from "@/lib/tx/create-experiment";
import type { CoordinatorConfig, JobSpec } from "@/lib/types";

type State = { stage: PublishStage | "idle"; error: UiError | null };

export function usePublish(account: UiWalletAccount, config: CoordinatorConfig) {
  const signer = useWalletUiSigner({ account });
  const router = useRouter();
  const [state, setState] = useState<State>({ stage: "idle", error: null });
  const onStage = (stage: PublishStage) => setState({ stage, error: null });
  const publish = (spec: JobSpec, economics: Economics) => {
    onStage("uploading");
    publishExperiment({ spec, economics, signer, config, onStage })
      .then(({ experiment }) => router.push(`/e/${experiment}`))
      .catch((error: unknown) => setState({ stage: "idle", error: publishError(error, config) }));
  };

  return { ...state, publish };
}

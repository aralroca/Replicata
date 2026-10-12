// Shapes of the coordinator API (docs/API.md). Amounts are token base units.

export type SpecInput = { url: string; sha256: string; path: string };

export type JobSpec = {
  version: number;
  title: string;
  description: string;
  image: string;
  image_digest: string;
  cmd: string[];
  env: Record<string, string>;
  inputs: SpecInput[];
  cpus: number;
  mem_mb: number;
  timeout_s: number;
  /** u64: kept as bigint so canonical JSON matches Rust bit for bit. */
  seed: bigint;
};

export type ExperimentStatus = "open" | "replicated" | "unresolved" | "cancelled";

export type ExperimentPhase = "claiming" | "revealing" | "finalizable" | "settled";

export type ClaimState = "claimed" | "committed" | "revealed" | "paid" | "slashed" | "refunded";

export type ClaimView = {
  worker: string;
  state: ClaimState;
  commitment: string;
  revealed_hash: string | null;
  claimed_at: number;
  has_bundle: boolean;
};

export type ExperimentView = {
  id: string;
  authority: string;
  mint: string;
  vault: string;
  nonce: number;
  spec_hash: string;
  image_digest: string;
  reward: number;
  bond: number;
  k: number;
  max_claims: number;
  claims_count: number;
  commits_count: number;
  reveals_count: number;
  commit_deadline: number;
  reveal_deadline: number;
  created_at: number;
  status: ExperimentStatus;
  phase: ExperimentPhase;
  result_hash: string | null;
  agree_count: number;
  replicators: string[];
  access_price: number;
  author_share_bps: number;
  escrowed: number;
  royalties_paid: number;
  royalties_pending: number;
  spec: (Omit<JobSpec, "seed"> & { seed: number }) | null;
  claims: ClaimView[];
};

export type CoordinatorConfig = {
  cluster: "localnet" | "devnet";
  network: string;
  rpc_url: string;
  program_id: string;
  mint: string;
  decimals: number;
  treasury: string;
  publish_fee: number;
  facilitator_url: string;
  faucets: { sol: string; usdc: string };
};

export type WorkerView = {
  worker: string;
  claims: number;
  ok: number;
  bad: number;
  earned: number;
  history: { experiment: string; state: ClaimState }[];
};

export type AuthorView = {
  author: string;
  experiments: ExperimentView[];
  royalties_paid: number;
  royalties_pending: number;
};

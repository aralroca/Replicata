// Deterministic coordinator data for the mock server (shapes from docs/API.md).

export const NETWORK = "solana:Fte3CqCT5A5K9Lb88mvQfc9ULeSuamKZ"; // a localnet-style CAIP-2 id
export const MINT = "717kPMNjzrCJ5KPEUiYVmon6fpC6wbTzQAGJDKQGuxsd";
export const FEE_PAYER = "GbhQKaZ41UqRebmjpkLfq8w6s7rs25KZcA4CM51rz7eF";
export const AUTHOR = "6TiLSzsZbHT8AziLjYXf8foiL5Zk3MymhUMaWBXG1vCT";
export const WORKERS = [
  "5H8cfQiAYbf8Pmn2Sq17x5Y4S4wz34cSW1MXSsFJ4Cbe",
  "DwMh8vYERsdg2d8eV7xEM8M9xAj2P7WAXvrdqkkPv7kX",
  "FBTtmAzbP9NexauGoLFHKPkYKAZLo7RpztWVDWrgyin4",
];

/** Experiment ids by scenario; the paid-result behaviour of the mock depends on them. */
export const IDS = {
  revealing: "azyKPKeBCdd8dBq9V9XiQf1mdR3SNwmMHJ5Dd3mSJmh", // open → 409 not_replicated
  noFunds: "7Gs7Gy2mVWV5FRBcXb3ATuwTjQEbXKfXUdftDPZXaQrX", // replicated → 402 insufficient_funds after paying
  paid: "94SwKKTFKzAXEPXsr5YQEjsGkTD4abRP4JoYBAq2kSZJ", // replicated → 200 bundle
  down: "EJh55mLkwhBsRqHDF5tMXtwkFxx8iLgCNJibgn5qwRZh", // replicated → 503 facilitator unavailable
};

const HASH = "cb8420d7fe1f37f793a6a57a1f4d001a0bf6db2bf296f9046ca32e764fa29776";

const SPEC = {
  version: 1,
  title: "",
  description: "Fixed-seed Monte-Carlo estimate of π, single-threaded.",
  image: "ghcr.io/aralroca/replicata-pi",
  image_digest: `sha256:${"0f".repeat(32)}`,
  cmd: ["python", "/app/main.py"],
  env: { PYTHONHASHSEED: "0" },
  inputs: [],
  cpus: 1,
  mem_mb: 512,
  timeout_s: 300,
  seed: 42,
};

const claim = (worker, state, revealed) => ({
  worker,
  state,
  commitment: "aa".repeat(32),
  revealed_hash: revealed ? HASH : null,
  claimed_at: 1_791_797_000,
  has_bundle: true,
});

const BASE = {
  authority: AUTHOR,
  mint: MINT,
  vault: "6h4mYSwNV2ShPi4LkG4THGw8mUMNzHAT3GUF7Rmbb7DQ",
  nonce: 1,
  spec_hash: "89eb8ce9dd78b94bb4e6255e121919f9fe82d10fbf3d1adf84e0f7001dded23b",
  image_digest: "0f".repeat(32),
  reward: 3_000_000,
  bond: 1_000_000,
  k: 3,
  max_claims: 3,
  commit_deadline: 1_791_798_000,
  reveal_deadline: 1_791_798_600,
  created_at: 1_791_797_000,
  access_price: 10_000,
  author_share_bps: 7_000,
  escrowed: 6_000_000,
};

const base = (id, title) => ({ ...BASE, id, spec: { ...SPEC, title } });

const revealing = () => ({
  ...base(IDS.revealing, "Monte-Carlo π (in progress)"),
  ...{ claims_count: 3, commits_count: 3, reveals_count: 1, status: "open", phase: "revealing" },
  ...{ result_hash: null, agree_count: 0, replicators: [], royalties_paid: 0, royalties_pending: 0 },
  claims: [claim(WORKERS[0], "revealed", true), claim(WORKERS[1], "committed"), claim(WORKERS[2], "committed")],
});

const replicated = (id, title) => ({
  ...base(id, title),
  ...{ claims_count: 3, commits_count: 3, reveals_count: 3, status: "replicated", phase: "settled" },
  ...{ result_hash: HASH, agree_count: 3, replicators: WORKERS, royalties_paid: 20_000, royalties_pending: 10_000 },
  claims: WORKERS.map((worker) => claim(worker, "paid", true)),
});

export function experiments() {
  return [
    revealing(),
    replicated(IDS.noFunds, "Lorenz attractor in integers"),
    replicated(IDS.paid, "Cellular automaton rule 110"),
    replicated(IDS.down, "Integer sieve benchmark"),
  ];
}

const CONFIG = {
  cluster: "localnet",
  network: NETWORK,
  program_id: "G9Ssnp8rjg7gpi4Sm3EZLoC73z9pdXd9etJzUDH39Wj9",
  mint: MINT,
  decimals: 6,
  treasury: "BdxqHU8v8nohpc3BWQMuvoLp34qrH5TJaBZfr72zw8Tr",
  publish_fee: 100_000,
  facilitator_url: "http://127.0.0.1:8402",
  faucets: { sol: "https://faucet.solana.com", usdc: "https://faucet.circle.com" },
};

export function config(rpcUrl) {
  return { ...CONFIG, rpc_url: rpcUrl };
}

export function worker(pubkey) {
  return {
    worker: pubkey,
    claims: 3,
    ok: 2,
    bad: 1,
    earned: 2_000_000,
    history: [{ experiment: IDS.paid, state: "paid" }],
  };
}

export function author(pubkey) {
  return { author: pubkey, experiments: experiments(), royalties_paid: 60_000, royalties_pending: 30_000 };
}

const ACCEPT = { scheme: "exact", network: NETWORK, amount: "10000", maxTimeoutSeconds: 300, asset: MINT };

/** Same shape as the real coordinator's PAYMENT-REQUIRED header (x402 v2). */
export function paymentRequired(id, resourceUrl) {
  return {
    x402Version: 2,
    error: "Payment-Signature header is required",
    resource: { url: resourceUrl, description: "Replicata result", mimeType: "application/gzip" },
    accepts: [{ ...ACCEPT, payTo: id, extra: { feePayer: FEE_PAYER } }],
  };
}

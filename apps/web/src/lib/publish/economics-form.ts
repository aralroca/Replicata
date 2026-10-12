// Step 2 of the publish wizard: replication & payment parameters. Mirrors the program's checks
// (`ZeroReward`, `InvalidReplicationParams`, `InvalidAuthorShare`, `InvalidDeadlines`).
import { BPS_DENOMINATOR, MAX_CLAIMS } from "@/generated";
import { parseAmount, USDC_DECIMALS } from "@/lib/amounts";
import type { SpecIssue } from "@/lib/spec/validate";
import type { Economics } from "@/lib/tx/create-experiment";

// The program's own limits (generated from the IDL), so UI validation cannot drift from it.
export { MAX_CLAIMS };

export type EconomicsForm = {
  k: string;
  maxClaims: string;
  bond: string;
  reward: string;
  commitMinutes: string;
  revealMinutes: string;
  accessPrice: string;
  authorSharePct: string;
};

export const DEFAULT_ECONOMICS_FORM: EconomicsForm = {
  k: "3",
  maxClaims: "3",
  bond: "1",
  reward: "3",
  commitMinutes: "30",
  revealMinutes: "30",
  accessPrice: "0.01",
  authorSharePct: "70",
};

const int = (text: string) => (/^\d+$/.test(text.trim()) ? Number(text.trim()) : NaN);

export function toEconomics(form: EconomicsForm, decimals = USDC_DECIMALS): Economics {
  return {
    reward: parseAmount(form.reward, decimals) ?? -1n,
    bond: parseAmount(form.bond, decimals) ?? -1n,
    accessPrice: parseAmount(form.accessPrice, decimals) ?? -1n,
    k: int(form.k),
    maxClaims: int(form.maxClaims),
    commitSecs: int(form.commitMinutes) * 60,
    revealSecs: int(form.revealMinutes) * 60,
    authorShareBps: Math.round(Number(form.authorSharePct) * 100),
  };
}

const RULES: { field: keyof EconomicsForm; message: string; ok: (e: Economics) => boolean }[] = [
  { field: "reward", message: "Reward must be greater than zero", ok: (e) => e.reward > 0n },
  { field: "bond", message: "Bond must be a USDC amount (0 or more)", ok: (e) => e.bond >= 0n },
  {
    field: "k",
    message: `Need 1 ≤ k ≤ max claims ≤ ${MAX_CLAIMS}`,
    ok: (e) => e.k >= 1 && e.k <= e.maxClaims && e.maxClaims <= MAX_CLAIMS,
  },
  {
    field: "commitMinutes",
    message: "Commit and reveal windows must be at least 1 minute",
    ok: (e) => e.commitSecs > 0 && e.revealSecs > 0,
  },
  {
    field: "accessPrice",
    message: "Access price must be greater than zero",
    ok: (e) => e.accessPrice > 0n,
  },
  {
    field: "authorSharePct",
    message: "Author share must be between 0% and 100%",
    ok: (e) => Number.isInteger(e.authorShareBps) && e.authorShareBps >= 0 && e.authorShareBps <= BPS_DENOMINATOR,
  },
];

export function economicsIssues(form: EconomicsForm, decimals = USDC_DECIMALS): SpecIssue[] {
  const economics = toEconomics(form, decimals);

  return RULES.filter((rule) => !rule.ok(economics)).map(({ field, message }) => ({
    field,
    message,
  }));
}

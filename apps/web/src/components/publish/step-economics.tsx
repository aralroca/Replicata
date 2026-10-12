import { binder } from "@/components/publish/bind";
import { FieldList, type FieldSpec } from "@/components/publish/field-list";
import { MAX_CLAIMS, type EconomicsForm } from "@/lib/publish/economics-form";
import type { SpecIssue } from "@/lib/spec/validate";

type Props = { form: EconomicsForm; setForm: (form: EconomicsForm) => void; issues: SpecIssue[] };

const ROWS: FieldSpec<EconomicsForm>[][] = [
  [
    { key: "k", label: "Required matches (k)", inputMode: "numeric" },
    { key: "maxClaims", label: "Worker slots", inputMode: "numeric", hint: `At most ${MAX_CLAIMS}.` },
    { key: "bond", label: "Worker bond (USDC)", inputMode: "decimal", hint: "Lost if the output disagrees." },
  ],
  [
    { key: "reward", label: "Reward (USDC)", inputMode: "decimal", hint: "Escrowed, split among matching workers." },
    { key: "accessPrice", label: "Access price (USDC)", inputMode: "decimal", hint: "Per download of the result." },
    {
      key: "authorSharePct",
      label: "Author share (%)",
      inputMode: "decimal",
      hint: "Of each access; rest to workers.",
    },
  ],
  [
    { key: "commitMinutes", label: "Commit window (minutes)", inputMode: "numeric" },
    { key: "revealMinutes", label: "Reveal window (minutes)", inputMode: "numeric" },
  ],
];

export function StepEconomics({ form, setForm, issues }: Props) {
  const bind = binder(form, setForm, issues);

  return (
    <fieldset className="grid gap-4">
      <legend className="mb-2 text-lg font-semibold">2 · Replication & payment</legend>
      {ROWS.map((row) => (
        <FieldList key={row[0].key} fields={row} bind={bind} className="grid gap-4 sm:grid-cols-3" />
      ))}
    </fieldset>
  );
}

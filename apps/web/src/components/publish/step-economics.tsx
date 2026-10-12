import { binder } from "@/components/publish/bind";
import { Field } from "@/components/publish/field";
import { MAX_CLAIMS, type EconomicsForm } from "@/lib/publish/economics-form";
import type { SpecIssue } from "@/lib/spec/validate";

type Props = { form: EconomicsForm; setForm: (form: EconomicsForm) => void; issues: SpecIssue[] };

type Bind = ReturnType<typeof binder<EconomicsForm>>;

function ReplicationFields({ bind }: { bind: Bind }) {
  return (
    <div className="grid gap-4 sm:grid-cols-3">
      <Field label="Required matches (k)" inputMode="numeric" {...bind("k")} />
      <Field label="Worker slots" inputMode="numeric" hint={`At most ${MAX_CLAIMS}.`} {...bind("maxClaims")} />
      <Field label="Worker bond (USDC)" inputMode="decimal" hint="Lost if the output disagrees." {...bind("bond")} />
    </div>
  );
}

function MoneyFields({ bind }: { bind: Bind }) {
  return (
    <div className="grid gap-4 sm:grid-cols-3">
      <Field label="Reward (USDC)" inputMode="decimal" hint="Escrowed now, split among matching workers." {...bind("reward")} />
      <Field label="Access price (USDC)" inputMode="decimal" hint="Paid per download of the verified result." {...bind("accessPrice")} />
      <Field label="Author share (%)" inputMode="decimal" hint="Of every access; the rest goes to replicators." {...bind("authorSharePct")} />
    </div>
  );
}

function WindowFields({ bind }: { bind: Bind }) {
  return (
    <div className="grid gap-4 sm:grid-cols-2">
      <Field label="Commit window (minutes)" inputMode="numeric" {...bind("commitMinutes")} />
      <Field label="Reveal window (minutes)" inputMode="numeric" {...bind("revealMinutes")} />
    </div>
  );
}

export function StepEconomics({ form, setForm, issues }: Props) {
  const bind = binder(form, setForm, issues);

  return (
    <fieldset className="grid gap-4">
      <legend className="mb-2 text-lg font-semibold">2 · Replication & payment</legend>
      <ReplicationFields bind={bind} />
      <MoneyFields bind={bind} />
      <WindowFields bind={bind} />
    </fieldset>
  );
}

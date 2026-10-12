import { errorsFor } from "@/hooks/use-publish-wizard";
import type { SpecIssue } from "@/lib/spec/validate";

/** Wires a form object to `<Field>` props: `bind("title")` → name/value/onChange/errors. */
export function binder<T extends Record<string, string>>(
  form: T,
  setForm: (form: T) => void,
  issues: SpecIssue[]
) {
  return (key: keyof T & string) => ({
    name: key,
    value: form[key],
    errors: errorsFor(issues, key),
    onChange: (value: string) => setForm({ ...form, [key]: value }),
  });
}

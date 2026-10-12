import type { ChangeEvent } from "react";

export type FieldProps = {
  name: string;
  label: string;
  value: string;
  onChange: (value: string) => void;
  errors?: string[];
  hint?: string;
  multiline?: boolean;
  placeholder?: string;
  inputMode?: "numeric" | "decimal" | "text";
};

type Change = ChangeEvent<HTMLInputElement | HTMLTextAreaElement>;

function describedBy({ name, hint, errors = [] }: FieldProps) {
  const ids = [hint ? `${name}-hint` : "", errors.length ? `${name}-error` : ""].filter(Boolean);

  return ids.length ? ids.join(" ") : undefined;
}

function Control(props: FieldProps) {
  const { name, value, placeholder, errors = [] } = props;
  const aria = { "aria-invalid": errors.length > 0, "aria-describedby": describedBy(props) };
  const common = { id: name, name, value, placeholder, className: "input mono", ...aria };
  const onChange = (event: Change) => props.onChange(event.target.value);

  return props.multiline ? (
    <textarea rows={3} {...common} onChange={onChange} />
  ) : (
    <input inputMode={props.inputMode} {...common} onChange={onChange} />
  );
}

function Note({ id, text, className }: { id: string; text?: string; className: string }) {
  return text ? (
    <p id={id} className={`text-xs ${className}`}>
      {text}
    </p>
  ) : null;
}

export function Field(props: FieldProps) {
  return (
    <div className="grid gap-1">
      <label htmlFor={props.name} className="text-sm font-medium">
        {props.label}
      </label>
      <Control {...props} />
      <Note id={`${props.name}-hint`} text={props.hint} className="text-muted" />
      <Note id={`${props.name}-error`} text={props.errors?.join(" · ")} className="text-danger" />
    </div>
  );
}

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

function describedBy(name: string, hint?: string, errors: string[] = []) {
  const ids = [hint ? `${name}-hint` : "", errors.length ? `${name}-error` : ""].filter(Boolean);

  return ids.length ? ids.join(" ") : undefined;
}

function Control(props: FieldProps) {
  const { name, value, multiline, placeholder, inputMode, errors = [] } = props;
  const common = {
    id: name,
    name,
    value,
    placeholder,
    className: "input mono",
    "aria-invalid": errors.length > 0,
    "aria-describedby": describedBy(name, props.hint, errors),
    onChange: (event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) => props.onChange(event.target.value),
  };

  return multiline ? <textarea rows={3} {...common} /> : <input inputMode={inputMode} {...common} />;
}

function Messages({ name, hint, errors = [] }: Pick<FieldProps, "name" | "hint" | "errors">) {
  return (
    <>
      {hint && <p id={`${name}-hint`} className="text-xs text-muted">{hint}</p>}
      {errors.length > 0 && <p id={`${name}-error`} className="text-xs text-danger">{errors.join(" · ")}</p>}
    </>
  );
}

export function Field(props: FieldProps) {
  return (
    <div className="grid gap-1">
      <label htmlFor={props.name} className="text-sm font-medium">
        {props.label}
      </label>
      <Control {...props} />
      <Messages name={props.name} hint={props.hint} errors={props.errors} />
    </div>
  );
}

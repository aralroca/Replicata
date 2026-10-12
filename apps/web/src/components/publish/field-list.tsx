import { Field, type FieldProps } from "@/components/publish/field";

export type FieldSpec<T> = { key: keyof T & string } & Omit<FieldProps, "name" | "value" | "onChange" | "errors">;

type Bind<T> = (key: keyof T & string) => Pick<FieldProps, "name" | "value" | "onChange" | "errors">;

type FieldListProps<T> = { fields: FieldSpec<T>[]; bind: Bind<T>; className: string };

/** Renders a declarative list of fields bound to a form object. */
export function FieldList<T>({ fields, bind, className }: FieldListProps<T>) {
  return (
    <div className={className}>
      {fields.map(({ key, ...field }) => (
        <Field key={key} {...field} {...bind(key)} />
      ))}
    </div>
  );
}

import type { ReactNode } from "react";

/** Page title + lead paragraph + content, the layout shared by the static pages. */
export function PageShell({ title, lead, children }: { title: string; lead: string; children: ReactNode }) {
  return (
    <div className="grid gap-6">
      <header>
        <h1 className="text-2xl font-bold">{title}</h1>
        <p className="mt-2 text-muted">{lead}</p>
      </header>
      {children}
    </div>
  );
}

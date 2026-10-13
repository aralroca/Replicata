import type { ReactNode } from "react";

/** Page title + lead paragraph + content, the layout shared by the static pages. */
export function PageShell({ title, lead, children }: { title: string; lead: string; children: ReactNode }) {
  return (
    <div className="grid gap-6">
      <header>
        <h1 className="text-4xl sm:text-5xl">{title}</h1>
        <p className="mt-3 max-w-2xl text-lg text-muted">{lead}</p>
      </header>
      {children}
    </div>
  );
}

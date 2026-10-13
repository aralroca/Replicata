import { REPLICATA_PROGRAM_ADDRESS } from "@/generated";
import { explorerUrl } from "@/lib/explorer";

const LINKS = [
  { href: "https://github.com/aralroca/Replicata", label: "GitHub" },
  { href: "https://github.com/aralroca/Replicata/blob/main/docs/API.md", label: "API" },
  { href: explorerUrl("address", REPLICATA_PROGRAM_ADDRESS), label: "Program" },
];

function FooterLink({ href, label }: { href: string; label: string }) {
  return (
    <a href={href} target="_blank" rel="noreferrer" className="text-muted no-underline hover:text-foreground">
      {label}
    </a>
  );
}

function FooterLinks() {
  return (
    <nav aria-label="Footer" className="flex gap-6">
      {LINKS.map((link) => (
        <FooterLink key={link.label} {...link} />
      ))}
    </nav>
  );
}

export function Footer() {
  return (
    <footer className="border-t border-border">
      <div className="mx-auto flex max-w-6xl flex-wrap items-center justify-between gap-4 px-6 py-8 text-sm text-muted">
        <p>Replicata · verifiable, paid replication on Solana</p>
        <FooterLinks />
      </div>
    </footer>
  );
}

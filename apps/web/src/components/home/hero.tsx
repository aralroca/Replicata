import Link from "next/link";

const PITCH =
  "Put a USDC reward on your experiment. Independent workers re-run it in a sealed sandbox, a majority of matching hashes seals the result on Solana, and every paid access becomes royalties for you and your replicators.";

function Eyebrow() {
  return (
    <Link href="/docs" className="eyebrow no-underline hover:no-underline">
      <span className="h-1.5 w-1.5 rounded-full bg-accent" aria-hidden />
      Escrow on Solana · Payments over x402
      <span aria-hidden>↗</span>
    </Link>
  );
}

function Actions() {
  return (
    <div className="mt-10 flex flex-wrap justify-center gap-3">
      <Link href="/publish" className="btn btn-primary">
        Publish an experiment <span aria-hidden>↗</span>
      </Link>
      <Link href="/start" className="btn btn-ghost">
        Replicate & earn <span aria-hidden>↗</span>
      </Link>
    </div>
  );
}

function Headline() {
  return (
    <h1 className="mt-8 max-w-4xl text-5xl leading-[1.05] sm:text-7xl">
      Results you can <span className="text-accent">verify.</span>
      <br />
      Replication that <span className="text-accent">pays.</span>
    </h1>
  );
}

export function Hero() {
  return (
    <section className="flex flex-col items-center py-12 text-center sm:py-20">
      <Eyebrow />
      <Headline />
      <p className="mt-6 max-w-2xl text-lg leading-relaxed text-muted">{PITCH}</p>
      <Actions />
    </section>
  );
}

import Link from "next/link";

import { ExperimentFeed } from "@/components/experiment-feed";

const PITCH =
  "Science has a reproducibility crisis and nobody is paid to check. Put a USDC reward on your experiment: independent workers re-run it, a majority of matching hashes seals the result on Solana, and every paid access becomes royalties for you and your replicators.";

function Actions() {
  return (
    <div className="mt-5 flex flex-wrap gap-3">
      <Link href="/publish" className="btn btn-primary">
        Publish an experiment
      </Link>
      <Link href="/start" className="btn btn-ghost">
        Replicate & earn
      </Link>
    </div>
  );
}

function Hero() {
  return (
    <section className="mb-10">
      <h1 className="text-3xl font-bold tracking-tight sm:text-4xl">Results you can verify. Replication that pays.</h1>
      <p className="mt-3 max-w-2xl text-muted">{PITCH}</p>
      <Actions />
    </section>
  );
}

function Feed() {
  return (
    <section aria-labelledby="feed-title">
      <h2 id="feed-title" className="mb-4 text-xl font-semibold">
        Latest experiments
      </h2>
      <ExperimentFeed />
    </section>
  );
}

export default function HomePage() {
  return (
    <>
      <Hero />
      <Feed />
    </>
  );
}

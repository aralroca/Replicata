import { ExperimentFeed } from "@/components/experiment-feed";
import { Hero } from "@/components/home/hero";
import { HowItWorks } from "@/components/home/how-it-works";

function Feed() {
  return (
    <section aria-labelledby="feed-title" className="py-12">
      <h2 id="feed-title" className="mb-6 text-3xl">
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
      <HowItWorks />
      <Feed />
    </>
  );
}

import { Hero } from "./components/Hero";
import { HowItWorks } from "./components/HowItWorks";
import { Features } from "./components/Features";
import { Install } from "./components/Install";
import { Footer } from "./components/Footer";
import { GitHubStarBadge } from "./components/GitHubStarBadge";

export default function App() {
  return (
    <div className="min-h-screen">
      <GitHubStarBadge />
      <Hero />
      <HowItWorks />
      <Features />
      <Install />
      <Footer />
    </div>
  );
}

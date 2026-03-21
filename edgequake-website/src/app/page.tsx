import { ArchitectureSection } from "@/components/sections/architecture";
import { BenchmarksSection } from "@/components/sections/benchmarks";
import { EcosystemSection } from "@/components/sections/ecosystem";
import { EnterpriseCTA } from "@/components/sections/enterprise-cta";
import { Hero } from "@/components/sections/hero";
import { ProblemSection } from "@/components/sections/problem";
import { QuickStartSection } from "@/components/sections/quickstart";
import { SolutionSection } from "@/components/sections/solution";

export default function Home() {
  return (
    <>
      <Hero />
      <ProblemSection />
      <SolutionSection />
      <ArchitectureSection />
      <BenchmarksSection />
      <QuickStartSection />
      <EcosystemSection />
      <EnterpriseCTA />
    </>
  );
}

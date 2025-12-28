import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // ============================================================================
  // Build Performance Optimization
  // Prevents CPU overload during compilation
  // ============================================================================

  // Limit experimental workers to prevent CPU overload
  experimental: {
    // Reduce worker count to prevent memory/CPU exhaustion
    cpus: Math.min(4, typeof process !== "undefined" && process.env.CI ? 2 : 4),
    // Use SWC minifier (faster than Terser)
    webpackBuildWorker: true,
  },

  // TypeScript configuration
  typescript: {
    // Don't fail build on TS errors (we use tsc separately)
    ignoreBuildErrors: false,
  },

  // Enable Turbopack for faster builds (Next.js 16+)
  // Turbopack is enabled by default with `next build` in Next.js 16

  // Output configuration
  output: "standalone",

  // Reduce logging
  logging: {
    fetches: {
      fullUrl: false,
    },
  },
};

export default nextConfig;

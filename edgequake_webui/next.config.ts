import type { NextConfig } from "next";
import { resolveDevProxyBackend } from "./src/lib/server/dev-proxy-backend";

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

  // Dev proxy: browser uses relative /api/v1 (same origin as :3001 UI).
  // Avoids NEXT_PUBLIC_API_URL port drift when backend auto-selects :8081.
  async rewrites() {
    if (process.env.NODE_ENV !== "development") {
      return [];
    }
    const backend = resolveDevProxyBackend();
    return [
      { source: "/api/:path*", destination: `${backend}/api/:path*` },
      { source: "/health", destination: `${backend}/health` },
      { source: "/ready", destination: `${backend}/ready` },
      { source: "/live", destination: `${backend}/live` },
      { source: "/ws/:path*", destination: `${backend}/ws/:path*` },
    ];
  },
};

export default nextConfig;

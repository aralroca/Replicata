import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  reactStrictMode: true,
  // E2E builds bake mock URLs into NEXT_PUBLIC_*; keep them out of the regular `.next`.
  distDir: process.env.NEXT_DIST_DIR ?? ".next",
};

export default nextConfig;

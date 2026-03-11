import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // Enable standalone output for Docker
  output: 'standalone',
  compiler: {
    // Remove ALL console statements in production builds
    // Keep them only for development (localhost)
    removeConsole: process.env.NODE_ENV === 'production' ? true : false
  },
  images: {
    remotePatterns: [
      {
        protocol: 'https',
        hostname: 'img.logo.dev',
        port: '',
        pathname: '/**',
      },
    ],
  },
  async headers() {
    return [
      {
        // Keep cross-origin isolation for API routes only.
        // Embeds like YouTube need these headers relaxed on page routes.
        source: '/api/:path*',
        headers: [
          {
            key: 'Cross-Origin-Embedder-Policy',
            value: 'require-corp',
          },
          {
            key: 'Cross-Origin-Opener-Policy',
            value: 'same-origin',
          },
        ],
      },
    ];
  },
};

export default nextConfig;

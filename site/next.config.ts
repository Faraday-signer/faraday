import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  /* faraday.to is the canonical host; fold www into it so search engines
   * see one site instead of two duplicates. */
  async redirects() {
    return [
      {
        source: "/:path*",
        has: [{ type: "host", value: "www.faraday.to" }],
        destination: "https://faraday.to/:path*",
        permanent: true,
      },
    ];
  },
};

export default nextConfig;

import type { MetadataRoute } from "next";

/* /flash is kept crawlable on purpose: it carries a noindex meta tag, and
 * crawlers must be able to fetch the page to see it. */
export default function robots(): MetadataRoute.Robots {
  return {
    rules: {
      userAgent: "*",
      allow: "/",
    },
    sitemap: "https://faraday.to/sitemap.xml",
  };
}

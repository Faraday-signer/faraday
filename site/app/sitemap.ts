import { readdirSync } from "node:fs";
import { join } from "node:path";
import type { MetadataRoute } from "next";

const BASE = "https://faraday.to";

/** Routes that exist but are deliberately noindexed, so they stay out. */
const EXCLUDED = new Set(["/flash"]);

/** Walk app/ for page.tsx files so new pages join the sitemap without
 * anyone remembering to edit this file. Route groups add no segment;
 * dynamic segments can't be enumerated and are skipped. */
function discoverRoutes(dir: string, route = ""): string[] {
  const entries = readdirSync(dir, { withFileTypes: true });
  const routes = entries.some((e) => e.isFile() && e.name === "page.tsx")
    ? [route || "/"]
    : [];
  for (const entry of entries) {
    if (!entry.isDirectory() || entry.name.startsWith("[")) continue;
    const next = entry.name.startsWith("(")
      ? route
      : `${route}/${entry.name}`;
    routes.push(...discoverRoutes(join(dir, entry.name), next));
  }
  return routes;
}

export default function sitemap(): MetadataRoute.Sitemap {
  return discoverRoutes(join(process.cwd(), "app"))
    .filter((route) => !EXCLUDED.has(route))
    .sort()
    .map((route) => ({
      url: route === "/" ? BASE : `${BASE}${route}`,
      lastModified: new Date(),
    }));
}

import { manifestAssetUrl, RELEASE_API_URL } from "@/lib/flash";

// The manifest is tiny and the release tag moves slowly, so cache the two
// upstream fetches for an hour. This also keeps us well inside GitHub's
// unauthenticated API rate limit (the `/releases/latest` call).
const REVALIDATE_SECONDS = 3600;

export async function GET() {
  const releaseRes = await fetch(RELEASE_API_URL, {
    headers: {
      Accept: "application/vnd.github+json",
      "User-Agent": "faraday.to",
    },
    next: { revalidate: REVALIDATE_SECONDS },
  });
  if (!releaseRes.ok) {
    return Response.json(
      { error: "Could not resolve the latest Faraday release." },
      { status: 502 },
    );
  }

  const release = (await releaseRes.json()) as { tag_name?: string };
  const tag = release.tag_name;
  if (!tag || !tag.startsWith("v")) {
    return Response.json(
      { error: "No versioned Faraday release found." },
      { status: 502 },
    );
  }
  const version = tag.slice(1);

  const manifestRes = await fetch(manifestAssetUrl(version), {
    next: { revalidate: REVALIDATE_SECONDS },
  });
  if (!manifestRes.ok) {
    return Response.json(
      { error: "Firmware manifest unavailable for the latest release." },
      { status: 502 },
    );
  }

  const manifest = (await manifestRes.json()) as {
    new_install_improv_wait_time?: number;
    builds?: { parts?: { path?: string }[] }[];
  };

  // Faraday firmware has no Wi-Fi and no Improv Wi-Fi Serial stack, so ESP Web
  // Tools' Improv probe would only ever log "Improv Wi-Fi Serial not detected"
  // and "Error fetching current state: TIMEOUT" — false errors that show up in
  // the browser's issues panel. Disable the probe (0 = off) regardless of what
  // the published manifest says.
  manifest.new_install_improv_wait_time = 0;

  // The manifest's `path` points at the raw GitHub asset, which the browser
  // can't fetch (no CORS). Rewrite it to the same-origin firmware proxy.
  for (const build of manifest.builds ?? []) {
    for (const part of build.parts ?? []) {
      if (typeof part.path === "string") {
        part.path = `/api/flash/firmware/${version}`;
      }
    }
  }

  // This response must patch into the browser immediately (it's the live
  // manifest, and it's what ESP Web Tools re-fetches before flashing, not just
  // what the app data cache would serve). No-store prevents a stale copy from
  // keeping the Improv probe on after we've disabled it, and once the site
  // proxies it, it must not be cached by the browser/CDN, because the file is
  // the "always latest release" contract.
  return Response.json(manifest, {
    headers: { "Cache-Control": "no-store" },
  });
}

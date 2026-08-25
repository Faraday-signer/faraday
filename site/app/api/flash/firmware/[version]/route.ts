import { firmwareAssetUrl, isValidVersion } from "@/lib/flash";

export async function GET(
  _request: Request,
  { params }: { params: Promise<{ version: string }> },
) {
  const { version } = await params;

  if (!isValidVersion(version)) {
    return new Response("Invalid version", { status: 400 });
  }

  const res = await fetch(firmwareAssetUrl(version), {
    next: { revalidate: 86400 },
  });
  if (!res.ok) {
    return new Response("Firmware not found", { status: 404 });
  }

  const body = await res.arrayBuffer();
  return new Response(body, {
    headers: {
      "Content-Type": "application/octet-stream",
      "Content-Length": String(body.byteLength),
      // Versioned and immutable: each release has its own URL, so this never
      // serves stale bytes for the version it points at.
      "Cache-Control": "public, max-age=31536000, immutable",
    },
  });
}
/**
 * Shared helpers for the web flasher (FA-13). The firmware and its ESP Web
 * Tools manifest are published as GitHub release assets (FA-17); the site
 * proxies them through same-origin API routes because GitHub's
 * `release-assets.githubusercontent.com` CDN does not send CORS headers, so a
 * browser fetch straight from faraday.to would be blocked.
 *
 * Release assets are named by the tag with its leading `v` stripped, e.g.
 * `v0.1.0` -> `faraday_esp32-s3.0.1.0.{bin,manifest.json}`.
 */

export const REPO = "Faraday-signer/faraday";

export const RELEASE_API_URL = `https://api.github.com/repos/${REPO}/releases/latest`;

// Matches the `<version>` used in asset filenames (semver plus optional
// prerelease/build suffix). Keeps the version safe to splice into a URL:
// no slashes, no `..`, no whitespace.
const VERSION_PATTERN = /^[0-9]+\.[0-9]+\.[0-9]+(?:[-+][0-9A-Za-z.-]+)?$/;

export function isValidVersion(version: string): boolean {
  return VERSION_PATTERN.test(version);
}

export function firmwareAssetUrl(version: string): string {
  return `https://github.com/${REPO}/releases/download/v${version}/faraday_esp32-s3.${version}.bin`;
}

export function manifestAssetUrl(version: string): string {
  return `https://github.com/${REPO}/releases/download/v${version}/faraday_esp32-s3.${version}.manifest.json`;
}

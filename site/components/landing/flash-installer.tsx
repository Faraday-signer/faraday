"use client";

import { useEffect, useState } from "react";

const GITHUB_RELEASES_URL =
  "https://github.com/Faraday-signer/faraday/releases/latest";

/**
 * The browser flasher (FA-13): an <esp-web-install-button> that flashes the
 * ESP32-S3 firmware over WebSerial. The library registers the custom element as
 * a module side-effect and touches `customElements` at import time, so it is
 * loaded dynamically inside an effect to keep it out of the server bundle.
 */
export function FlashInstaller() {
  const [ready, setReady] = useState(false);

  useEffect(() => {
    let cancelled = false;
    // Deep-import the pre-bundled build: the package's `main` entry imports
    // @material/web internals that don't resolve under our bundler, but
    // `dist/web/install-button.js` is self-contained (its dialog and stub
    // flashers load as sibling chunks).
    import("esp-web-tools/dist/web/install-button.js")
      .then(() => {
        if (!cancelled) setReady(true);
      })
      .catch(() => {
        // Leave `ready` false; the manual-flash fallback below still renders.
      });
    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <div className="mt-12">
      {ready ? (
        <esp-web-install-button manifest="/api/flash/manifest">
          <button
            slot="activate"
            className="inline-flex h-12 cursor-pointer items-center justify-center gap-2 rounded-sm bg-primary px-6 font-mono text-sm uppercase tracking-[0.14em] text-brand transition-colors hover:bg-primary/90 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background"
          >
            Connect device
          </button>

          <div slot="unsupported" className="space-y-4">
            <p className="rounded-sm border border-brand-ink/40 bg-brand/10 p-4 text-sm leading-relaxed text-foreground/85">
              This browser can&apos;t talk to USB devices over WebSerial. Open
              this page in <strong className="font-semibold">Chrome</strong> or{" "}
              <strong className="font-semibold">Edge</strong> on a desktop
              (WebSerial doesn&apos;t work on iOS), or flash the board manually.
            </p>
            <ManualFlash />
          </div>

          <div slot="not-allowed" className="space-y-4">
            <p className="rounded-sm border border-brand-ink/40 bg-brand/10 p-4 text-sm leading-relaxed text-foreground/85">
              WebSerial is only available over HTTPS. Open this page at{" "}
              <code className="font-mono text-brand-ink">
                https://faraday.to/flash
              </code>{" "}
              to flash from the browser.
            </p>
            <ManualFlash />
          </div>
        </esp-web-install-button>
      ) : (
        <div className="flex w-full flex-col items-center justify-center gap-4 rounded-sm border border-border bg-muted/30 py-16">
          <p className="font-mono text-[11px] uppercase tracking-[0.18em] text-muted-foreground">
            Loading flasher…
          </p>
        </div>
      )}
    </div>
  );
}

function ManualFlash() {
  return (
    <div className="rounded-sm border border-border bg-muted/30 p-5">
      <p className="font-mono text-[11px] uppercase tracking-[0.16em] text-brand-ink">
        Flash manually with esptool
      </p>
      <ol className="mt-3 list-decimal space-y-2 pl-5 text-sm leading-relaxed text-foreground/75">
        <li>
          Download the firmware from the{" "}
          <a
            href={GITHUB_RELEASES_URL}
            target="_blank"
            rel="noopener noreferrer"
            className="text-brand-ink underline underline-offset-2 transition-colors hover:text-brand-ink/80"
          >
            latest GitHub release
          </a>
          .
        </li>
        <li>
          Install{" "}
          <a
            href="https://docs.espressif.com/projects/esptool/en/latest/esp32/"
            target="_blank"
            rel="noopener noreferrer"
            className="text-brand-ink underline underline-offset-2 transition-colors hover:text-brand-ink/80"
          >
            esptool
          </a>
          , then run:
        </li>
      </ol>
      <pre className="mt-3 overflow-x-auto rounded-sm border border-border bg-background p-4 font-mono text-xs leading-relaxed text-foreground/85">
        {`esptool.py --chip esp32s3 write_flash \\
  0x0 faraday_esp32-s3.<version>.bin`}
      </pre>
      <p className="mt-3 text-xs leading-relaxed text-foreground/60">
        The release notes list the exact versioned filename and a checksum to
        verify the download.
      </p>
    </div>
  );
}

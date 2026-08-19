# 2026-08-19 — FA-17 spike: ESP32-S3 release artifact + ESP Web Tools manifest

The release workflow can now build and publish a browser-flashable ESP32-S3
image. `release.yml` gained an `esp32-image` job (parallel to the Pi job) plus a
`publish` job that ships every asset in one `action-gh-release` call (no race
between the two parallel build jobs). Also fixed a latent Pi bug: the release
job shared `Swatinem/rust-cache` (`key: arm`) with CI, which could report the
build "fresh" without re-linking `faraday`, leaving the Buildroot bind-mount
pointing at nothing ("Faraday binary not found at /opt/faraday-bin").

## Spike findings (image format, toolchain, manifest)

**Merged image.** ESP Web Tools can't patch flash params on the fly, so ESP-IDF
firmware must be packed into a single binary before flashing from the browser.
Produced with:

```
cargo espflash save-image --release --merge --skip-padding --chip esp32s3 out.bin
```

- `--merge` concatenates bootloader + partition table + app at their ESP-IDF
  offsets; espflash derives the offsets from the build's partition table, so
  none are hand-coded in the workflow.
- `--skip-padding` drops the trailing pad-to-flash-size, keeping the asset small
  (~ app size + bootloader/partition-table, well under 2 MB); ESP Web Tools
  erases before writing, so the full-flash 0xFF tail is unnecessary.
- ESP32-S3 layout for reference: bootloader @ `0x0`, partition table @ `0x8000`,
  app @ `0x10000`. We do **not** list these in the manifest — one part at
  offset `0` (the merged image) is enough.

**CI toolchain.** Same as `ci.yml`'s `esp32-build` job (`esp-rs/xtensa-toolchain@v1.5.3`,
`buildtargets: esp32s3`, `ldproxy: true`) plus a host-stable `dtolnay/rust-toolchain@stable`
solely to `cargo install cargo-espflash --locked`. The `nm` radio-symbol audit
is repeated here so the released ELF (not just the PR branch) is gated on the
no-WiFi/BT-driver property.

**Manifest schema.** A minimal ESP Web Tools manifest, versioned with the tag:

```json
{
  "name": "Faraday",
  "version": "<version>",
  "new_install_prompt_erase": true,
  "builds": [
    { "chipFamily": "ESP32-S3", "parts": [ { "path": "faraday_esp32-s3.<version>.bin", "offset": 0 } ] }
  ]
}
```

The `path` is a bare filename — resolved relative to the manifest URL, and both
assets live side by side in `releases/download/v<tag>/`, so no absolute URL is
needed. GitHub release assets serve `Access-Control-Allow-Origin: *`, so the
page at faraday.to can fetch both.

## Not yet verified (needs hardware)

- **Flash size / mode.** The merged image inherits them from the ESP-IDF
  `sdkconfig` baked into the bootloader (`sdkconfig.defaults` does not pin
  `CONFIG_ESPTOOLPY_FLASHSIZE` or `CONFIG_ESPTOOLPY_FLASHMODE`). The Waveshare
  ESP32-S3-Touch-LCD-2 has **16 MB** flash (ESP32-S3R8, 8 MB PSRAM). If the
  ESP-IDF default (4 MB/DIO) works for the boot, fine — but this should be
  confirmed on hardware, and if needed pinned via
  `CONFIG_ESPTOOLPY_FLASHSIZE_16MB=y` (+ the correct `CONFIG_ESPTOOLPY_FLASHMODE_*`)
  in `sdkconfig.defaults`. Cargo note: `espflash save-image` also honors an
  `espflash.toml` `[flash]` block if we decide the CLI-level size matters.
- **Acceptance #2 — flash a real board** from the *published* release assets
  (not a local build) and confirm it boots. Not runnable here (no device).

## Files

- `.github/workflows/release.yml` — 3-job restructuring (`pi-image`,
  `esp32-image`, `publish`).
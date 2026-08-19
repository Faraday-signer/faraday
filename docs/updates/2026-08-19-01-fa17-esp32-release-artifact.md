# 2026-08-19 — FA-17 spike: ESP32-S3 release artifact + ESP Web Tools manifest

The release workflow can now build and publish a browser-flashable ESP32-S3
image. `release.yml` gained an `esp32-image` job (parallel to the Pi job) plus a
`publish` job that ships every asset in one `action-gh-release` call (no race
between the two parallel build jobs).

## Two bugs found and fixed along the way

**Pi: the ARM binary was never where the mount looked.** `raspberry-pi` is a
workspace member, so `cargo zigbuild` writes the binary to the *workspace-root*
`target/`, not `raspberry-pi/target/`. The compose bind-mount (and the
`test -f` guard) pointed at `raspberry-pi/target/…` — an empty path — so the
Buildroot container died with "Faraday binary not found at /opt/faraday-bin".
Fixed by repointing both the `docker-compose.yml` mount and the workflow guard
at `./target/arm-unknown-linux-gnueabihf/release/faraday`.

**ESP32-S3: firmware (~2.4 MB) doesn't fit the default 1 MB factory partition.**
`espflash save-image` errored `image_too_big`. Fixed with a custom
`partitions.csv` (4 MB factory) passed to espflash via `--partition-table`, and
`CONFIG_ESPTOOLPY_FLASHSIZE_16MB=y` in `sdkconfig.defaults` so the bootloader's
flash-size header matches the board.

## Spike findings (image format, toolchain, manifest)

**Merged image.** ESP Web Tools can't patch flash params on the fly, so ESP-IDF
firmware must be packed into a single binary before flashing from the browser.
Produced with:

```
cargo espflash save-image --release --merge --skip-padding \
  --chip esp32s3 --partition-table partitions.csv out.bin
```

- `--merge` concatenates bootloader + partition table + app at their ESP-IDF
  offsets (derived from the partition table, not hand-coded).
- `--partition-table partitions.csv` overrides the 1 MB default factory
  partition (esp-idf-sys convention: no `CONFIG_PARTITION_TABLE_*` in
  `sdkconfig.defaults` — pass the CSV to espflash instead).
- `--skip-padding` drops the trailing pad-to-flash-size, keeping the asset small
  (bootloader + partition table + app ≈ 2.5 MB); ESP Web Tools erases before
  writing, so the full-flash 0xFF tail is unnecessary.
- ESP32-S3 layout for reference: bootloader @ `0x0`, partition table @ `0x8000`,
  app @ `0x10000`. The manifest still lists just one part at offset `0` (the
  merged image).

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

## Not yet verified

- **Flash mode.** Pinned size to 16 MB (board-accurate); flash *mode* is left at
  the ESP-IDF default (DIO), which is the compatible baseline. If the board's
  flash supports QIO and we want the throughput, pin `CONFIG_ESPTOOLPY_FLASHMODE_QIO`
  — but confirm the physical part first; setting QIO on a DIO-only part fails to
  boot.
- **Acceptance #2 — flash a real board** from the *published* release assets
  (not a local build) and confirm it boots. Not runnable here (no device).

## Files

- `.github/workflows/release.yml` — 3 jobs (`pi-image`, `esp32-image`, `publish`).
- `docker-compose.yml` — ARM binary mount repointed at the workspace-root target dir.
- `esp32-touch2/partitions.csv` — new: 4 MB factory partition table.
- `esp32-touch2/sdkconfig.defaults` — pins 16 MB flash.
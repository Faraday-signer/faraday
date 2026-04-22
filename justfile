# Faraday command shortcuts — run `just` to list, `just <recipe>` to run.

# Run the desktop simulator (webcam + GUI window).
sim:
    cargo run --features simulator

# Cross-compile the ARM binary for Pi Zero.
arm:
    cargo zigbuild --release --target arm-unknown-linux-gnueabihf

# Build the full Pi OS image (cold Buildroot — slow).
image: arm
    docker compose up

# Rebuild the Pi OS image, reusing warm Buildroot state.
image-fast: arm
    BUILD_ARGS='--pi0 --no-clean' docker compose up

# Flash the latest Pi image to the SD card at /dev/disk9.
flash:
    diskutil unmountDisk /dev/disk9
    sudo dd if=images/faraday_os.pi0.img of=/dev/rdisk9 bs=4m status=progress
    diskutil eject /dev/disk9

# Build the browser extension (Chromium MV3 → extension/.output/chrome-mv3).
ext:
    cd extension && npm run build

# Install extension dependencies.
ext-install:
    cd extension && npm install

# Run cargo tests.
test:
    cargo test

# Type-check both simulator and Pi targets.
check:
    cargo check --features simulator
    cargo check --release --target arm-unknown-linux-gnueabihf

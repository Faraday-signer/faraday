# Faraday command shortcuts — run `just` to list, `just <recipe>` to run.

# Run the desktop simulator (webcam + GUI window).
sim:
    cd raspberry-pi && cargo run --features simulator

# Cross-compile the ARM binary for Pi Zero.
arm:
    cd raspberry-pi && cargo zigbuild --release --target arm-unknown-linux-gnueabihf

# Build the full Pi OS image (cold Buildroot — slow).
image: arm
    docker compose up

# Rebuild the Pi OS image, reusing warm Buildroot state.
image-fast: arm
    BUILD_ARGS='--pi0 --no-clean' docker compose up

# Flash the latest Pi image to an SD card. Pass DEVICE=/dev/diskN (find with `diskutil list`).
flash DEVICE:
    diskutil unmountDisk {{DEVICE}}
    sudo dd if=images/faraday_os.pi0.img of={{replace(DEVICE, "/dev/disk", "/dev/rdisk")}} bs=4m status=progress
    diskutil eject {{DEVICE}}

# Build the browser extension (Chromium MV3 → extension/.output/chrome-mv3).
ext:
    cd extension && npm run build

# Install extension dependencies.
ext-install:
    cd extension && npm install

# Run cargo tests.
test:
    cd raspberry-pi && cargo test

# Type-check both simulator and Pi targets.
check:
    cd raspberry-pi && cargo check --features simulator
    cd raspberry-pi && cargo check --release --target arm-unknown-linux-gnueabihf

# Build the ESP32-S3 firmware.
esp:
    cd esp32 && cargo build --release

# Build and flash the ESP32-S3 firmware with serial monitor.
esp-flash:
    cd esp32 && cargo run --release

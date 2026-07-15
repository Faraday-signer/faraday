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

# Build the ESP32-S3-Touch-LCD-2 firmware.
esp-touch2:
    cd esp32-touch2 && cargo build --release

# Build and flash the ESP32-S3-Touch-LCD-2 firmware with serial monitor.
esp-touch2-flash:
    cd esp32-touch2 && cargo run --release

# Future ESP32 boards get their own crate + recipes, e.g.:
# esp-lcd35:
#     cd esp32-lcd35 && cargo build --release

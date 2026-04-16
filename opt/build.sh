#!/bin/bash
# =============================================================================
# Faraday OS Build Script
# Buildroot cross-compilation for Pi Zero
# =============================================================================

set -o errexit -o pipefail
export FORCE_UNSAFE_CONFIGURE=1

cur_dir=$(pwd)

# Clone Buildroot if not present
BUILDROOT_VERSION="2024.11"
if [ ! -d "${cur_dir}/buildroot" ]; then
  echo "Cloning Buildroot ${BUILDROOT_VERSION}..."
  git clone --depth 1 --branch "${BUILDROOT_VERSION}" https://github.com/buildroot/buildroot.git "${cur_dir}/buildroot"
fi

help() {
  echo "Usage: build.sh [options]"
  echo "  --pi0       Build for Pi Zero / Zero W"
  echo "  --no-clean  Keep previous build files"
  echo "  --no-op     Keep container alive without building"
  exit 2
}

tail_endless() {
  echo "No-op mode. Container alive. Use 'docker exec' to interact."
  tail -f /dev/null
  exit 0
}

install_app() {
  local rootfs_overlay=$1

  rm -rf ${rootfs_overlay}/opt/

  # Copy Rust binary
  mkdir -p ${rootfs_overlay}/opt/
  if [ -f "/opt/faraday-bin" ]; then
    echo "Copying Faraday binary to overlay"
    cp /opt/faraday-bin ${rootfs_overlay}/opt/faraday
    chmod +x ${rootfs_overlay}/opt/faraday
  else
    echo "ERROR: Faraday binary not found at /opt/faraday-bin"
    echo "Build with: cargo zigbuild --release --target arm-unknown-linux-gnueabihf"
    exit 1
  fi

  # Create startup script
  cat > ${rootfs_overlay}/opt/start.sh << 'STARTUP'
#!/bin/sh
/opt/faraday &
STARTUP
  chmod +x ${rootfs_overlay}/opt/start.sh

  # Create init script
  mkdir -p ${rootfs_overlay}/etc/init.d
  cat > ${rootfs_overlay}/etc/init.d/S99faraday << 'INITD'
#!/bin/sh
case "$1" in
  start)
    /opt/start.sh
    ;;
  stop)
    killall faraday 2>/dev/null
    ;;
esac
INITD
  chmod +x ${rootfs_overlay}/etc/init.d/S99faraday

  echo "App ready in ${rootfs_overlay}/opt/"
}

build_image() {
  local config_name="${1:-pi0}"
  local config_dir="./${config_name}"
  local rootfs_overlay="./rootfs-overlay"
  local build_dir="${cur_dir}/../output"
  local image_dir="${cur_dir}/../images"

  if [ ! -d "${config_dir}" ]; then
    echo "Config ${config_name} not found"
    exit 1
  fi

  if [ "${2}" != "no-clean" ]; then
    rm -rf "${build_dir:?}"/* 2>/dev/null || true
    mkdir -p "${build_dir}"
  fi

  install_app "${rootfs_overlay}"

  # Run Buildroot
  PATH="/usr/lib/ccache:${PATH}" make BR2_EXTERNAL="../${config_dir}/" O="${build_dir}" -C ./buildroot/ "${config_name}_defconfig"
  cd "${build_dir}"
  PATH="/usr/lib/ccache:${PATH}" make

  # Move image to output
  mkdir -p "${image_dir}"
  if [ -f "${build_dir}/images/faraday_os.img" ]; then
    mv -f "${build_dir}/images/faraday_os.img" "${image_dir}/faraday_os.${config_name}.img"
    echo ""
    echo "============================================="
    echo "  Build complete!"
    echo "============================================="
    echo "  Image: ${image_dir}/faraday_os.${config_name}.img"
    echo "  Size:  $(du -h "${image_dir}/faraday_os.${config_name}.img" | cut -f1)"
    echo ""
    sha256sum "${image_dir}/faraday_os.${config_name}.img" || true
  else
    echo "Build failed - no image produced"
    ls -la "${build_dir}/images/" 2>/dev/null || true
    exit 1
  fi

  cd - > /dev/null
}

# Parse arguments
NO_OP=false
PI0=false
CLEAN_ARG="clean"

while (( "$#" )); do
  case "$1" in
    --pi0) PI0=true; shift ;;
    --no-clean) CLEAN_ARG="no-clean"; shift ;;
    --no-op) NO_OP=true; shift ;;
    -h|--help) help ;;
    *) echo "Unknown: $1"; help ;;
  esac
done

if $NO_OP; then
  tail_endless
fi

if $PI0; then
  build_image "pi0" "${CLEAN_ARG}"
fi

exit 0

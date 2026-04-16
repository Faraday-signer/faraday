#!/bin/bash
# Post-image script - creates the SD card image

set -e

sectorsToBlocks() {
  echo $(( ( "$1" * 512 ) / 1024 ))
}

sectorsToBytes() {
  echo $(( "$1" * 512 ))
}

export disk_timestamp="2026/04/14T12:00:00"

rm -rf ${BUILD_DIR}/custom_image
mkdir -p ${BUILD_DIR}/custom_image
cd ${BUILD_DIR}/custom_image

# Create disk image
dd if=/dev/zero of=disk.img bs=1M count=120

# Partition table - single FAT32 partition, bootable
/sbin/sfdisk disk.img <<EOF
  label: dos
  label-id: 0x50151627

  disk.img1 : type=c, bootable
EOF

# Format FAT32
START=$(/sbin/fdisk -l -o Start disk.img|tail -n 1)
SECTORS=$(/sbin/fdisk -l -o Sectors disk.img|tail -n 1)
/sbin/mkfs.vfat --invariant -i 50151627 -n FARADAYOS disk.img --offset $START $(sectorsToBlocks $SECTORS)
OFFSET=$(sectorsToBytes $START)

# Copy boot files
mkdir -p boot/overlays overlays
cp ${BASE_DIR}/images/rpi-firmware/cmdline.txt boot/cmdline.txt
cp ${BASE_DIR}/images/rpi-firmware/config.txt boot/config.txt
cp ${BASE_DIR}/images/rpi-firmware/bootcode.bin boot/bootcode.bin
cp ${BASE_DIR}/images/rpi-firmware/fixup_x.dat boot/fixup_x.dat
cp ${BASE_DIR}/images/rpi-firmware/start_x.elf boot/start_x.elf
cp ${BASE_DIR}/images/rpi-firmware/overlays/* overlays/
cp ${BASE_DIR}/images/*.dtb boot/
cp ${BASE_DIR}/images/zImage boot/zImage

chmod 0755 $(find boot overlays)
touch -d "${disk_timestamp}" $(find boot overlays)

# Write to image using mtools
mcopy -bpm -i "disk.img@@$OFFSET" boot/* ::
mcopy -bpm -i "disk.img@@$OFFSET" overlays/* ::overlays
mv disk.img ${BASE_DIR}/images/faraday_os.img

cd -

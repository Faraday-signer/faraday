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

# Stage boot files. boot_config.txt references no dtoverlays, so we
# deliberately skip copying the overlays directory — nothing would read it.
mkdir -p boot
cp ${BASE_DIR}/images/rpi-firmware/cmdline.txt boot/cmdline.txt
cp ${BASE_DIR}/images/rpi-firmware/config.txt boot/config.txt
cp ${BASE_DIR}/images/rpi-firmware/bootcode.bin boot/bootcode.bin
cp ${BASE_DIR}/images/rpi-firmware/fixup_x.dat boot/fixup_x.dat
cp ${BASE_DIR}/images/rpi-firmware/start_x.elf boot/start_x.elf
cp ${BASE_DIR}/images/bcm2708-rpi-zero*.dtb boot/
cp ${BASE_DIR}/images/zImage boot/zImage

chmod 0755 $(find boot)
touch -d "${disk_timestamp}" $(find boot)

# Size the disk to actual content + headroom, aligned to MiB. The
# initramfs lives inside zImage so content never grows at runtime —
# 3 MiB headroom is plenty. Below 33 MiB mkfs.vfat auto-selects FAT16,
# which the Pi boot firmware reads just fine.
CONTENT_BYTES=$(du -sb boot | awk '{print $1}')
HEADROOM_MIB=3
DISK_MIB=$(( (CONTENT_BYTES + 1024 * 1024 - 1) / (1024 * 1024) + HEADROOM_MIB ))
if [ ${DISK_MIB} -lt 16 ]; then DISK_MIB=16; fi
echo "Boot content: $(( CONTENT_BYTES / 1024 )) KiB, disk image: ${DISK_MIB} MiB"

dd if=/dev/zero of=disk.img bs=1M count=${DISK_MIB}

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

# Write staged files to the FAT partition
mcopy -bpm -i "disk.img@@$OFFSET" boot/* ::
mv disk.img ${BASE_DIR}/images/faraday_os.img

cd -

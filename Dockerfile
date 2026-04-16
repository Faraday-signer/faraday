# Faraday OS Builder
# Debian 12 + Buildroot cross-compilation for Pi Zero

FROM debian:12

RUN apt-get -qq update && apt-get -y install \
    locales lsb-release git wget make binutils gcc g++ patch \
    gzip bzip2 perl tar cpio unzip rsync file bc libssl-dev \
    build-essential libncurses-dev mtools fdisk dosfstools ccache \
    kpartx e2fsprogs

# Locale
RUN locale-gen en_US.UTF-8
ENV LANG=en_US.UTF-8
ENV LANGUAGE=en_US:en
ENV LC_ALL=en_US.UTF-8

WORKDIR /opt
ENTRYPOINT ["./build.sh"]

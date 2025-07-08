# nvidia/cuda:12.8.0-cudnn8-runtime-ubuntu22.04
ARG RUNTIMEBASE=nvidia/cuda@sha256:05de765c12d993316f770e8e4396b9516afe38b7c52189bce2d5b64ef812db58
# nvidia/cuda:12.8.0-cudnn8-devel-ubuntu22.04
ARG DEVELBASE=nvidia/cuda@sha256:2a015be069bda4de48d677b6e3f271a2794560c7d788a39a18ecf218cae0751d

FROM $DEVELBASE AS exopticon-build

WORKDIR /exopticon

ENV DEBIAN_FRONTEND=noninteractive

# Install system packages
#RUN echo 'deb http://http.debian.net/debian bullseye main contrib non-free' >> /etc/apt/sources.list
RUN apt-get update && apt-get install --no-install-recommends -y \
  # Exopticon Build Dependencies
  bzip2 unzip \
  dpkg-dev file imagemagick libz3-dev libc6-dev \
  libcurl4-openssl-dev libdb-dev libevent-dev libffi-dev\
  libgdbm-dev libgeoip-dev libglib2.0-dev libjpeg-dev \
  libkrb5-dev liblzma-dev libmagickcore-dev libmagickwand-dev\
  libncurses5-dev libncursesw5-dev libpng-dev libpq-dev \
  libreadline-dev libsqlite3-dev libssl-dev libtool libwebp-dev \
  libxml2-dev libxslt-dev libyaml-dev make patch xz-utils \
  zlib1g-dev default-libmysqlclient-dev \
  curl python3-pil python3-lxml \
  python3 python3-dev python3-pip python3-setuptools python3-wheel \
  git libopencv-dev python3-opencv python3-scipy cmake \
  mold clang \
  # ffmpeg
  ffmpeg libavformat-dev libswscale-dev libavutil-dev libavcodec-dev libavfilter-dev \
  # hwaccel
  intel-media-va-driver-non-free i965-va-driver-shaders \
  # gstreamer
  libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev \
  gstreamer1.0-plugins-base gstreamer1.0-plugins-good \
  gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly \
  gstreamer1.0-libav libgstrtspserver-1.0-dev libges-1.0-dev \
  libgstreamer-plugins-bad1.0-dev \
  && apt-get clean \
  && rm -rf /var/lib/apt/lists/*

# # Install nvdec headers
# RUN git clone https://git.videolan.org/git/ffmpeg/nv-codec-headers.git \
#     && cd nv-codec-headers \
#     && make \
#     && make install

# # Build ffmpeg
# RUN apt-get update && apt-get -y build-dep ffmpeg \
#     && git clone https://github.com/FFmpeg/FFmpeg -b master ffmpeg \
#     && cd ffmpeg && git checkout f1357274e912b40928ed4dc100b4c1de8750508b # just the latest commit at this time

# RUN cd ffmpeg && ./configure \
#        --disable-sdl2 \
#        --disable-alsa \
#        --disable-xlib \
#        --disable-sndio \
#        --disable-libxcb \
#        --disable-libxcb-shm \
#        --disable-libxcb-xfixes \
#        --disable-libxcb-shape \
#        --disable-libass \
#        --enable-cuvid \
#        --enable-nvdec \
#        --enable-gpl \
#        --enable-vaapi \
#        --enable-libfreetype \
#        --enable-libmp3lame \
#        --enable-libopus \
#        --enable-libtheora \
#        --enable-libvorbis \
#        --enable-libvpx \
#        --enable-libx264 \
#        --enable-shared \
#     && make -j`getconf _NPROCESSORS_ONLN` \
#     && make install

# install node.js and npm
RUN mkdir /node && cd /node \
    && curl https://nodejs.org/dist/v20.11.1/node-v20.11.1-linux-x64.tar.xz > node.tar.xz \
    && tar xf node.tar.xz \
    && mv node*/* . \
    && rm -rf node.tar.xz
ENV PATH=/node/bin:$PATH

RUN mkdir /cargo && mkdir /rust
RUN chown 1000:1000 /cargo /rust

# configure run user
RUN groupadd -r -g 1000 exopticon && useradd --no-log-init -m -g exopticon -G plugdev --uid 1000 exopticon
RUN chown exopticon:exopticon /exopticon

USER exopticon:plugdev

ENV CARGO_HOME=/cargo
ENV RUST_HOME=/rust

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
  && /cargo/bin/rustup toolchain install 1.88.0 \
  && /cargo/bin/rustup default 1.88.0 \
  && /cargo/bin/rustup component add clippy

RUN pip3 install msgpack imutils numpy pathspec==0.9.0 dvc[s3]==1.11.16 importlib-metadata
RUN /home/exopticon/.local/bin/dvc config --global core.analytics false

ENV EXOPTICONWORKERS=/exopticon/target/debug/
ENV PYTHONPATH=$EXOPTICONWORKERS:/opt/opencv/lib/python3.7/dist-packages
ENV CUDA_HOME=/usr/local/cuda-12.2
ENV CUDA_PATH=/usr/local/cuda-12.2/bin
ENV CUDA_TOOLKIT_DIR=/usr/local/cuda-12.2
ENV CUDACXX=/usr/local/cuda-12.2/bin/nvcc
ENV PATH=$CUDA_PATH:/exopticon/target/debug:$CARGO_HOME/bin:/exopticon/exopticon/workers:/home/exopticon/.local/bin/:$PATH

WORKDIR /exopticon

FROM exopticon-build AS development
# This state is just used for local development
# configure environment

USER root

# Create volume mount paths and set ownership
RUN mkdir -p /cargo /exopticon/target \
 && chown exopticon:plugdev /cargo /exopticon/target

USER exopticon:plugdev

# configure mold linker
RUN mkdir ~/.cargo
RUN echo "[target.x86_64-unknown-linux-gnu]" >> ~/.cargo/config.toml
RUN echo "rustflags = [\"-C\", \"linker=clang\", \"-C\", \"link-arg=--ld-path=/usr/bin/mold\"]" >> ~/.cargo/config.toml

ENTRYPOINT ["tail", "-f", "/dev/null"]

FROM exopticon-build AS prod-build

USER exopticon:plugdev

COPY --chown=exopticon:exopticon . ./

RUN make ci-flow

FROM $RUNTIMEBASE AS exopticon-runtime
WORKDIR /exopticon

USER root

ENV FLASK_ENV=development
ENV DEBIAN_FRONTEND=noninteractive
# Install packages for apt repo
RUN apt-get -qq update \
# ffmpeg
  && apt-get install --no-install-recommends -y \
  bzip2 libssl3 \
  libbz2-1.0 libc6 \
  libcurl4 libevent-2.1-7 libffi7 \
  libgdbm6 libgeoip1 libglib2.0 \
  libkrb5-3 liblzma5 libmagickcore-6.q16-6 libmagickwand-6.q16-6 \
  libncurses5 libncursesw5 libpng16-16 libpq5 \
  libreadline8 libsqlite3-0 \
  libxml2 libxslt1.1 libyaml-0-2 \
  python3-opencv \
  # ffmpeg runtime deps
  ffmpeg \
  # Add imutils and numpy
  && apt-get install --no-install-recommends -y \
  python3-setuptools python3-pip python3-wheel python3-pillow python3-scipy \
  && pip3 install imutils numpy \
  && apt-get purge -y python3-setuptools python3-pip python3-wheel \
  # clean up
  && apt-get autoremove -y \
  && apt-get clean \
  && rm -rf /var/lib/apt/lists/* \ # /wheels \
  && (apt-get autoremove -y; apt-get autoclean -y)

# configure run user
RUN groupadd -r -g 1000 exopticon && useradd --no-log-init -m -g exopticon --uid 1000 exopticon
RUN chown exopticon:exopticon /exopticon

FROM exopticon-runtime AS exopticon-cuda

COPY --chown=exopticon:exopticon --from=prod-build /exopticon/target/release/exopticon .

COPY --chown=exopticon:exopticon --from=prod-build /exopticon/target/release/capture_worker .

ENV EXOPTICONWORKERS=/exopticon/
ENV PYTHONPATH=$EXOPTICONWORKERS:/opt/opencv/lib/python3.7/dist-packages
ENV PATH=/exopticon:$PATH
ENV LD_LIBRARY_PATH=/usr/local/lib

USER exopticon:plugdev

ENTRYPOINT /exopticon/exopticon

FROM dmattli/debian-cuda:10.0-buster-devel AS exopticon-build
WORKDIR /exopticon

ENV CC=gcc-7
ENV CXX=g++-7

# Install system packages
RUN echo 'deb-src http://http.debian.net/debian buster main contrib non-free' > /etc/apt/sources.list.d/src.list
RUN apt-get update && apt-get install --no-install-recommends -y \
  # Exopticon Build Dependencies
  libturbojpeg0-dev bzip2 \
  dpkg-dev file imagemagick libbz2-dev libc6-dev \
  libcurl4-openssl-dev libdb-dev libevent-dev libffi-dev\
  libgdbm-dev libgeoip-dev libglib2.0-dev libjpeg-dev \
  libkrb5-dev liblzma-dev libmagickcore-dev libmagickwand-dev\
  libncurses5-dev libncursesw5-dev libpng-dev libpq-dev \
  libreadline-dev libsqlite3-dev libssl-dev libtool libwebp-dev \
  libxml2-dev libxslt-dev libyaml-dev make patch xz-utils \
  zlib1g-dev default-libmysqlclient-dev libturbojpeg0-dev \
  curl python3-pil python3-lxml \
  python3 python3-pip python3-setuptools python3-wheel \
  git libopencv-dev python3-opencv cmake \
# ffmpeg \
  && apt-get clean \
  && rm -rf /var/lib/apt/lists/*

# Install nvdec headers
RUN git clone https://git.videolan.org/git/ffmpeg/nv-codec-headers.git \
    && cd nv-codec-headers \
    && make \
    && make install

# Build ffmpeg
RUN apt-get update && apt-get -y build-dep ffmpeg \
    && git clone https://github.com/FFmpeg/FFmpeg -b master ffmpeg \
    && cd ffmpeg && git checkout f1357274e912b40928ed4dc100b4c1de8750508b # just the latest commit at this time

RUN cd ffmpeg && ./configure \
       --disable-sdl2 \
       --disable-alsa \
       --disable-xlib \
       --disable-sndio \
       --disable-libxcb \
       --disable-libxcb-shm \
       --disable-libxcb-xfixes \
       --disable-libxcb-shape \
       --disable-libass \
       --enable-cuvid \
       --enable-nvdec \
       --enable-gpl \
       --enable-vaapi \
       --enable-libfreetype \
       --enable-libmp3lame \
       --enable-libopus \
       --enable-libtheora \
       --enable-libvorbis \
       --enable-libvpx \
       --enable-libx264 \
       --enable-shared \
    && make -j`getconf _NPROCESSORS_ONLN` \
    && make install

# Add Coral tpu repository and install python libraries
 RUN echo "deb https://packages.cloud.google.com/apt coral-edgetpu-stable main" | tee /etc/apt/sources.list.d/coral-edgetpu.list \
     && wget -O - https://packages.cloud.google.com/apt/doc/apt-key.gpg | apt-key add - \
     && apt-get update \
     && echo "libedgetpu1-max libedgetpu/accepted-eula select true" | debconf-set-selections \
     && apt-get -qq update && apt-get -qq install --no-install-recommends -y \
        libedgetpu1-max=15.0 python3-pycoral edgetpu-compiler \
     && apt-get clean \
     && rm -rf /var/lib/apt/lists/*

ENV FLASK_ENV=development
ENV DEBIAN_FRONTEND=noninteractive
# Install packages for apt repo
RUN apt-get -qq update \
#    && apt-get upgrade -y \
    && apt-get -qq install --no-install-recommends -y \
    gnupg wget unzip tzdata python3-gi \
    && apt-get -qq install --no-install-recommends -y \
        python3-pip \
#    && pip3 install -U /wheels/*.whl \
    && APT_KEY_DONT_WARN_ON_DANGEROUS_USAGE=DontWarn apt-key adv --fetch-keys https://packages.cloud.google.com/apt/doc/apt-key.gpg \
    && echo "deb https://packages.cloud.google.com/apt coral-edgetpu-stable main" > /etc/apt/sources.list.d/coral-edgetpu.list \
    && echo "libedgetpu1-max libedgetpu/accepted-eula select true" | debconf-set-selections \
    && apt-get -qq update && apt-get -qq install --no-install-recommends -y \
        libedgetpu1-max=15.0 \
    && rm -rf /var/lib/apt/lists/* \ # /wheels \
    && (apt-get autoremove -y; apt-get autoclean -y)


# install node.js and npm
RUN mkdir /node && cd /node \
    && wget https://nodejs.org/dist/v12.16.2/node-v12.16.2-linux-x64.tar.xz -O node.tar.xz \
    && tar xf node.tar.xz \
    && mv node*/* . \
    && rm -rf node.tar.xz
ENV PATH=$PATH:/node/bin

# configure gcc-7 as default for CUDA
RUN rm /usr/bin/gcc /usr/bin/g++ \
    && ln -s /usr/bin/gcc-7 /usr/bin/gcc \
    && ln -s /usr/bin/g++-7 /usr/bin/g++

RUN mkdir /cargo && mkdir /rust
RUN chown 1000:1000 /cargo /rust

# configure run user
RUN groupadd -r -g 1000 exopticon && useradd --no-log-init -m -g exopticon --uid 1000 exopticon
RUN adduser exopticon plugdev
RUN chown exopticon:exopticon /exopticon

USER exopticon:exopticon

ENV CARGO_HOME=/cargo
ENV RUST_HOME=/rust

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
  && /cargo/bin/rustup toolchain install 1.47.0 \
  && /cargo/bin/rustup default 1.47.0 \
  && /cargo/bin/rustup component add clippy \
  && /cargo/bin/cargo install --force cargo-make

RUN pip3 install msgpack imutils numpy dvc[ssh]
RUN /home/exopticon/.local/bin/dvc config --global core.analytics false

# configure environment
ENV EXOPTICONWORKERS=/exopticon/target/assets/workers
ENV PYTHONPATH=$EXOPTICONWORKERS:/opt/opencv/lib/python3.7/dist-packages
ENV PATH=/exopticon/target/debug:$CARGO_HOME/bin:/exopticon/exopticon/workers:/home/exopticon/.local/bin/:$PATH
ENV CUDA_HOME=/usr/local/cuda-10.0
ENV CUDA_PATH=/usr/local/cuda-10.0/bin
ENV CUDA_TOOLKIT_DIR=/usr/local/cuda-10.0
ENV CUDACXX=/usr/local/cuda-10.0/bin/nvcc

WORKDIR /exopticon

FROM exopticon-build AS prod-build

USER exopticon:exopticon

COPY --chown=exopticon:exopticon . ./

RUN dvc pull workers/yolov4/data/yolov4-tiny.weights \
      workers/coral/data/ssd_mobilenet_v2_coco_quant_postprocess_edgetpu.tflite

RUN cargo make --profile release build-release

FROM dmattli/debian-cuda:10.0-buster-runtime AS exopticon-runtime

WORKDIR /exopticon

USER root

RUN apt-get update && apt-get install --no-install-recommends -y \
  libturbojpeg0 bzip2 \
  libbz2-1.0 libc6 \
  libcurl4 libevent-2.1-6 libffi6 \
  libgdbm6 libgeoip1 libglib2.0 \
  libkrb5-3 liblzma5 libmagickcore-6.q16-6 libmagickwand-6.q16-6 \
  libncurses5 libncursesw5 libpng16-16 libpq5 \
  libreadline5 libsqlite3-0 libssl1.1 libwebp6 \
  libxml2 libxslt1.1 libyaml-0-2 \
  zlib1g libturbojpeg0 \
  python3-opencv \
  # ffmpeg runtime deps
  libxcb-shape0 libxcb-xfixes0 \
  && apt-get clean \
  && rm -rf /var/lib/apt/lists/*


# Add Coral tpu repository and install python libraries
ENV FLASK_ENV=development
ENV DEBIAN_FRONTEND=noninteractive
# Install packages for apt repo
RUN apt-get -qq update \
#    && apt-get upgrade -y \
    && apt-get -qq install --no-install-recommends -y \
    gnupg wget unzip tzdata python3-gi \
    && apt-get -qq install --no-install-recommends -y \
        python3-pip \
#    && pip3 install -U /wheels/*.whl \
    && APT_KEY_DONT_WARN_ON_DANGEROUS_USAGE=DontWarn apt-key adv --fetch-keys https://packages.cloud.google.com/apt/doc/apt-key.gpg \
    && echo "deb https://packages.cloud.google.com/apt coral-edgetpu-stable main" > /etc/apt/sources.list.d/coral-edgetpu.list \
    && echo "libedgetpu1-max libedgetpu/accepted-eula select true" | debconf-set-selections \
    && apt-get -qq update && apt-get -qq install --no-install-recommends -y \
        libedgetpu1-max=15.0 \
    && rm -rf /var/lib/apt/lists/* \ # /wheels \
    && (apt-get autoremove -y; apt-get autoclean -y)

RUN apt-get update && apt-get install --no-install-recommends -y \
      python3-setuptools python3-pip python3-wheel \
    && pip3 install msgpack imutils numpy \
    && apt-get purge -y python3-setuptools python3-pip python3-wheel \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# configure run user
RUN groupadd -r -g 1000 exopticon && useradd --no-log-init -m -g exopticon --uid 1000 exopticon
RUN chown exopticon:exopticon /exopticon

FROM exopticon-runtime

COPY --chown=exopticon:exopticon --from=prod-build /exopticon/target/release/exopticon .

COPY --chown=exopticon:exopticon --from=prod-build /exopticon/target/assets/workers ./workers

# Copy ffmpeg libraries
RUN mkdir -p /usr/local/lib /usr/local/bin \
  && apt-get install $(apt-cache depends ffmpeg | grep Depends | sed "s/.*ends:\ //" | tr '\n' ' ')
COPY --from=exopticon-build /usr/local/lib/lib* /usr/local/lib/
COPY --from=exopticon-build /usr/local/bin /usr/local/bin

ENV EXOPTICONWORKERS=/exopticon/workers/
ENV PYTHONPATH=$EXOPTICONWORKERS:/opt/opencv/lib/python3.7/dist-packages
ENV PATH=/exopticon:$PATH
ENV LD_LIBRARY_PATH=/usr/local/lib

USER exopticon

ENTRYPOINT /exopticon/exopticon

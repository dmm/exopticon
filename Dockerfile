FROM dmattli/debian-cuda:10.0-buster-devel AS exopticon-build
WORKDIR /exopticon

ENV CC=gcc-7
ENV CXX=g++-7

# Install system packages
RUN echo 'deb-src http://http.debian.net/debian buster main contrib non-free' > /etc/apt/sources.list.d/src.list
RUN apt-get update && apt-get install --no-install-recommends -y \
  # Exopticon Build Dependencies
  libturbojpeg0-dev bzip2 unzip \
  dpkg-dev file imagemagick libbz2-dev libc6-dev \
  libcurl4-openssl-dev libdb-dev libevent-dev libffi-dev\
  libgdbm-dev libgeoip-dev libglib2.0-dev libjpeg-dev \
  libkrb5-dev liblzma-dev libmagickcore-dev libmagickwand-dev\
  libncurses5-dev libncursesw5-dev libpng-dev libpq-dev \
  libreadline-dev libsqlite3-dev libssl-dev libtool libwebp-dev \
  libxml2-dev libxslt-dev libyaml-dev make patch xz-utils \
  zlib1g-dev default-libmysqlclient-dev libturbojpeg0-dev \
  curl python3-pil python3-lxml \
  python3 python3-dev python3-pip python3-setuptools python3-wheel \
  git libopencv-dev python3-opencv python3-scipy cmake \
  ffmpeg \
  && apt-get clean \
  && rm -rf /var/lib/apt/lists/*

# Add Coral tpu repository and install python libraries
 RUN echo "deb https://packages.cloud.google.com/apt coral-edgetpu-stable main" | tee /etc/apt/sources.list.d/coral-edgetpu.list \
     && wget -O - https://packages.cloud.google.com/apt/doc/apt-key.gpg | apt-key add - \
     && apt-get update \
     && echo "libedgetpu1-max libedgetpu/accepted-eula select true" | debconf-set-selections \
     && apt-get -qq update && apt-get -qq install --no-install-recommends -y \
        libedgetpu1-max=16.0 python3-pycoral edgetpu-compiler python3-gi \
     && apt-get clean \
     && rm -rf /var/lib/apt/lists/*

# install node.js and npm
RUN mkdir /node && cd /node \
    && wget https://nodejs.org/dist/v14.17.5/node-v14.17.5-linux-x64.tar.xz -O node.tar.xz \
    && tar xf node.tar.xz \
    && mv node*/* . \
    && rm -rf node.tar.xz
ENV PATH=/node/bin:$PATH

# configure gcc-7 as default for CUDA
RUN rm /usr/bin/gcc /usr/bin/g++ \
    && ln -s /usr/bin/gcc-7 /usr/bin/gcc \
    && ln -s /usr/bin/g++-7 /usr/bin/g++

# Install cargo-make
RUN mkdir cm && cd cm \
  && curl -L https://github.com/sagiegurari/cargo-make/releases/download/0.35.0/cargo-make-v0.35.0-x86_64-unknown-linux-musl.zip > cargo-make.zip \
  && echo "429c60665b20d43c6492045539add3f41a6339a0fb83d3d7d5bb66f926ccff36  cargo-make.zip" | sha256sum -c \
  && unzip cargo-make.zip && cp cargo-make-*/cargo-make /usr/local/bin/cargo-make \
  && cd .. && rm -r cm/

RUN mkdir /cargo && mkdir /rust
RUN chown 1000:1000 /cargo /rust

# configure run user
RUN groupadd -r -g 1000 exopticon && useradd --no-log-init -m -g exopticon -G plugdev --uid 1000 exopticon
RUN chown exopticon:exopticon /exopticon

USER exopticon:plugdev

ENV CARGO_HOME=/cargo
ENV RUST_HOME=/rust

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
  && /cargo/bin/rustup toolchain install 1.47.0 \
  && /cargo/bin/rustup default 1.47.0 \
  && /cargo/bin/rustup component add clippy
#RUN /cargo/bin/cargo uninstall --force cargo-make

RUN pip3 install msgpack imutils numpy dvc[s3]==1.11.16
RUN /home/exopticon/.local/bin/dvc config --global core.analytics false

ENV EXOPTICONWORKERS=/exopticon/target/assets/workers
ENV PYTHONPATH=$EXOPTICONWORKERS:/opt/opencv/lib/python3.7/dist-packages
ENV PATH=/exopticon/target/debug:$CARGO_HOME/bin:/exopticon/exopticon/workers:/home/exopticon/.local/bin/:$PATH
ENV CUDA_HOME=/usr/local/cuda-10.0
ENV CUDA_PATH=/usr/local/cuda-10.0/bin
ENV CUDA_TOOLKIT_DIR=/usr/local/cuda-10.0
ENV CUDACXX=/usr/local/cuda-10.0/bin/nvcc

WORKDIR /exopticon

FROM exopticon-build AS development
# This state is just used for local development
# configure environment

USER root

# Create volume mount paths and set ownership
RUN mkdir -p /cargo /exopticon/target \
 && chown exopticon:plugdev /cargo /exopticon/target

USER exopticon:plugdev

ENTRYPOINT ["tail", "-f", "/dev/null"]

FROM exopticon-build AS prod-build

USER exopticon:plugdev

COPY --chown=exopticon:exopticon . ./

RUN dvc pull workers/yolov4/data/yolov4-tiny.weights \
      workers/coral/data/ssd_mobilenet_v2_coco_quant_postprocess_edgetpu.tflite

RUN cargo make --profile release ci-flow

FROM debian:buster-slim AS exopticon

WORKDIR /exopticon

USER root

ENV FLASK_ENV=development
ENV DEBIAN_FRONTEND=noninteractive
# Install packages for apt repo
RUN apt-get -qq update \
# ffmpeg and runtime deps
  && apt-get install --no-install-recommends -y \
  libpq5 libturbojpeg0 ffmpeg python3-opencv \
# Add Coral tpu repository and install python libraries.net core
    && apt-get -qq install --no-install-recommends -y \
    gnupg wget unzip tzdata python3-gi \
    && apt-get -qq install --no-install-recommends -y \
        python3-pip \
    && APT_KEY_DONT_WARN_ON_DANGEROUS_USAGE=DontWarn apt-key adv --fetch-keys https://packages.cloud.google.com/apt/doc/apt-key.gpg \
    && echo "deb https://packages.cloud.google.com/apt coral-edgetpu-stable main" > /etc/apt/sources.list.d/coral-edgetpu.list \
    && echo "libedgetpu1-max libedgetpu/accepted-eula select true" | debconf-set-selections \
    && apt-get -qq update && apt-get -qq install --no-install-recommends -y \
        libedgetpu1-max=16.0 python3-pycoral \
    && apt-get purge -y python3-setuptools python3-pip python3-wheel gnupg wget unzip mono-runtime \
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

COPY --chown=exopticon:exopticon --from=prod-build /exopticon/target/release/exopticon .

COPY --chown=exopticon:exopticon --from=prod-build /exopticon/target/assets/workers ./workers

ENV EXOPTICONWORKERS=/exopticon/workers/
ENV PYTHONPATH=$EXOPTICONWORKERS:/opt/opencv/lib/python3.7/dist-packages
ENV PATH=/exopticon:$PATH
ENV LD_LIBRARY_PATH=/usr/local/lib

USER exopticon:plugdev

ENTRYPOINT /exopticon/exopticon

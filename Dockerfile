FROM dmattli/debian-cuda:10.0-buster-devel AS exopticon-build
WORKDIR /exopticon

ENV CC=gcc-7
ENV CXX=g++-7

# Install system packages
RUN echo 'deb-src http://http.debian.net/debian buster main contrib non-free' > /etc/apt/sources.list.d/src.list
RUN apt-get update && apt-get install --no-install-recommends -y \
  # Exopticon Build Dependencies
  libavcodec-dev libavformat-dev libswscale-dev libavfilter-dev \
  libavutil-dev libturbojpeg0-dev bzip2 \
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
  git libopencv-dev python3-opencv cmake ffmpeg \
  && apt-get clean \
  && rm -rf /var/lib/apt/lists/*

# Add Coral tpu repository and install python libraries
RUN echo "deb https://packages.cloud.google.com/apt coral-edgetpu-stable main" | tee /etc/apt/sources.list.d/coral-edgetpu.list \
    && wget -O - https://packages.cloud.google.com/apt/doc/apt-key.gpg | apt-key add - \
    && apt-get update \
    && apt-get install -y python3-pycoral edgetpu-compiler libedgetpu1-std \
    && mkdir temp && cd temp && apt-get download libedgetpu1-max \
    && ar x libedgetpu1-max* && tar xf data.tar.xz && cp usr/lib/x86_64-linux-gnu/libedgetpu.so.1.0 /usr/lib/x86_64-linux-gnu/ \
    && cd .. && rm -rf temp \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

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

RUN cargo make --profile release build-release

RUN dvc pull workers/yolov4/data/yolov4-tiny.weights \
      workers/coral/data/ssd_mobilenet_v2_coco_quant_postprocess_edgetpu.tflite

FROM dmattli/debian-cuda:10.0-buster-runtime AS exopticon-runtime

WORKDIR /exopticon

USER root

RUN apt-get update && apt-get install --no-install-recommends -y \
  libavcodec58 libavformat58 libswscale5 libavfilter7 \
  libavutil56 libturbojpeg0 bzip2 \
  libbz2-1.0 libc6 \
  libcurl4 libevent-2.1-6 libffi6 \
  libgdbm6 libgeoip1 libglib2.0 \
  libkrb5-3 liblzma5 libmagickcore-6.q16-6 libmagickwand-6.q16-6 \
  libncurses5 libncursesw5 libpng16-16 libpq5 \
  libreadline5 libsqlite3-0 libssl1.1 libwebp6 \
  libxml2 libxslt1.1 libyaml-0-2 \
  zlib1g libturbojpeg0 \
  python3-opencv \
  ffmpeg \
  && apt-get clean \
  && rm -rf /var/lib/apt/lists/*

# Add Coral tpu repository and install python libraries
RUN echo "deb https://packages.cloud.google.com/apt coral-edgetpu-stable main" | tee /etc/apt/sources.list.d/coral-edgetpu.list \
    && wget -O - https://packages.cloud.google.com/apt/doc/apt-key.gpg | apt-key add - \
    && apt-get update \
    && apt-get install -y python3-pycoral libedgetpu1-std xz-utils \
    && mkdir temp && cd temp && apt-get download libedgetpu1-max \
    && ar x libedgetpu1-max* && tar xf data.tar.xz && cp usr/lib/x86_64-linux-gnu/libedgetpu.so.1.0 /usr/lib/x86_64-linux-gnu/ \
    && cd .. && rm -rf temp \
    && apt-get purge -y xz-utils \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

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

ENV EXOPTICONWORKERS=/exopticon/workers/
ENV PYTHONPATH=$EXOPTICONWORKERS:/opt/opencv/lib/python3.7/dist-packages
ENV PATH=/exopticon:$PATH

USER exopticon

ENTRYPOINT /exopticon/exopticon

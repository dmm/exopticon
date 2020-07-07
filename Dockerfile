#FROM gw000/debian-cuda:9.1_7.0
FROM debian-cuda:10.0-buster-devel AS devel
WORKDIR /exopticon

ENV CC=gcc-7
ENV CXX=g++-7

# Install system packages
RUN echo 'deb-src http://http.debian.net/debian buster main contrib non-free' > /etc/apt/sources.list.d/src.list
RUN apt-get update && apt-get install --no-install-recommends -y \
  # Exopticon Build Dependencies
  libavcodec-dev libavformat-dev libswscale-dev libavfilter-dev \
  libavutil-dev libturbojpeg0-dev autoconf automake bzip2 \
  dpkg-dev file g++ gcc imagemagick libbz2-dev libc6-dev \
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
  && apt-get clean \
  && rm -rf /var/lib/apt/lists/*

# install dlib
## dlib dependencies
#RUN apt-get update && apt-get install --no-install-recommends -y \
#git \
#cmake \
#libsm6 \
#libxext6 \
#libxrender-dev \
#libopenblas-dev \
#liblapack-dev \
#lsb-release \
#&& apt-get clean \
#&& rm -rf /var/lib/apt/lists/*
#RUN pip3 install scikit-build

## fetch dlib sources
#RUN git clone -b 'v19.16' --single-branch https://github.com/davisking/dlib.git
#RUN mkdir -p /dlib/build
#RUN cmake -H/dlib -B/dlib/build -DDLIB_USE_CUDA=1 -DUSE_AVX_INSTRUCTIONS=1
#RUN cmake --build /dlib/build
#RUN cd /dlib; python3 /dlib/setup.py install

# install opencv
# RUN apt-get update && apt-get install -y --no-install-recommends libopenblas-dev python3-dev \
#     && apt-get build-dep -y --no-install-recommends opencv \
#     && apt-get clean \
#     && rm -rf /var/lib/apt/lists/* \
#     && git clone --branch 4.3.0 --depth 1 https://github.com/opencv/opencv.git /root/opencv \
#     && git clone --branch 4.3.0 --depth 1 https://github.com/opencv/opencv_contrib.git  /root/opencv-contrib \
#     && cd /root/opencv \
#     && git checkout tags/4.3.0 \
#     && mkdir build && cd build \
#     && cmake \
#     -DOPENCV_EXTRA_MODULES_PATH=/root/opencv-contrib/modules \
#     -DWITH_CUDA=ON \
#     -DCUDA_GENERATION="Pascal" \
#     -DOPENCV_DNN_CUDA=ON \
#     -DENABLE_FAST_MATH=1 \
#     -DCUDA_FAST_MATH=1 \
#     -DWITH_CUBLAS=1 \
#     -DBUILD_opencv_python3=yes \
#     -DBUILD_opencv_java=off \
#     -DCMAKE_BUILD_TYPE=RELEASE \
#     -DCMAKE_INSTALL_PREFIX=/opt/opencv \
#     -D BUILD_EXAMPLES=OFF \
#     -D BUILD_DOCS=OFF \
#     -D BUILD_PERF_TESTS=OFF \
#     -D BUILD_TESTS=OFF \
#     -D WITH_TBB=ON \
#     -D WITH_OPENMP=ON \
#     -D WITH_IPP=ON \
#     -D WITH_NVCUVID=ON \
#     -D WITH_OPENCL=ON \
#     -D WITH_CSTRIPES=ON \
#     .. \
#     && make -j20 \
#     && make install \
#     && cd && rm -r /root/opencv

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
RUN chown exopticon:exopticon /exopticon

USER exopticon:exopticon

ENV CARGO_HOME=/cargo
ENV RUST_HOME=/rust

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
  && /cargo/bin/rustup component add clippy \
  && /cargo/bin/rustup update \
  && /cargo/bin/rustup default stable

RUN pip3 install msgpack imutils numpy

# configure environment
ENV EXOPTICONWORKERS=/exopticon/workers/dist
ENV PYTHONPATH=$EXOPTICONWORKERS:/opt/opencv/lib/python3.7/dist-packages
ENV PATH=$CARGO_HOME/bin:/exopticon/exopticon/workers:$PATH
ENV CUDA_HOME=/usr/local/cuda-10.0
ENV CUDA_PATH=/usr/local/cuda-10.0/bin
ENV CUDA_TOOLKIT_DIR=/usr/local/cuda-10.0
ENV CUDACXX=/usr/local/cuda-10.0/bin/nvcc

FROM devel AS prod-build
WORKDIR /exopticon

COPY --chown=exopticon:exopticon . ./

RUN cargo build --release

FROM debian-cuda:10.0-buster-runtime
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
  python-setuptools python3-pip python3-opencv \
  && apt-get clean \
  && rm -rf /var/lib/apt/lists/*

RUN pip3 install setuptools && pip3 install msgpack imutils numpy

# configure run user
RUN groupadd -r -g 1000 exopticon && useradd --no-log-init -m -g exopticon --uid 1000 exopticon
RUN chown exopticon:exopticon /exopticon

COPY --chown=exopticon:exopticon --from=prod-build /exopticon/target/release/exopticon .

ENV EXOPTICONWORKERS=/exopticon/workers/
ENV PYTHONPATH=$EXOPTICONWORKERS:/opt/opencv/lib/python3.7/dist-packages

COPY --chown=exopticon:exopticon --from=prod-build /exopticon/workers/dist ./workers

USER exopticon

ENTRYPOINT /exopticon/exopticon

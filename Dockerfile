#FROM gw000/debian-cuda:9.1_7.0
FROM gw000/debian-cuda:10.3

ENV CC=gcc-7
ENV CXX=g++-7

# Install system packages
RUN echo 'deb-src http://http.debian.net/debian buster main contrib non-free' > /etc/apt/sources.list.d/src.list
RUN apt-get update && apt-get install --no-install-recommends -y \
# Exopticon Dependencies
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
           git \
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
RUN apt-get update && apt-get install -y --no-install-recommends libopenblas-dev python3-dev \
    && apt-get build-dep -y --no-install-recommends opencv \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* \
    && git clone --depth 1 --branch master https://github.com/opencv/opencv.git /root/opencv \
    && git clone --depth 1 --branch master https://github.com/opencv/opencv_contrib.git  /root/opencv-contrib \
    && cd /root/opencv \
    && mkdir build && cd build \
    && cmake \
    -DOPENCV_EXTRA_MODULES_PATH=/root/opencv-contrib/modules \
    -DWITH_CUDA=ON \
    -DCUDA_GENERATION="Pascal" \
    -DOPENCV_DNN_CUDA=ON \
    -DENABLE_FAST_MATH=1 \
    -DCUDA_FAST_MATH=1 \
    -DWITH_CUBLAS=1 \
    -DBUILD_opencv_python3=yes \
    -DBUILD_opencv_java=off \
    -DCMAKE_BUILD_TYPE=RELEASE \
    -DCMAKE_INSTALL_PREFIX=/opt/opencv \
    -D BUILD_EXAMPLES=OFF \
    -D BUILD_DOCS=OFF \
    -D BUILD_PERF_TESTS=OFF \
    -D BUILD_TESTS=OFF \
    -D WITH_TBB=ON \
    -D WITH_OPENMP=ON \
    -D WITH_IPP=ON \
    -D WITH_NVCUVID=ON \
    -D WITH_OPENCL=ON \
    -D WITH_CSTRIPES=ON \
    .. \
    && make -j20 \
    && make install \
    && cd && rm -r /root/opencv

# install node.js and npm
RUN mkdir /node && cd /node \
    && wget https://nodejs.org/dist/v12.16.2/node-v12.16.2-linux-x64.tar.xz -O node.tar.xz \
    && tar xf node.tar.xz \
    && rm -rf node.tar.xz
ENV PATH=$PATH:/node/node-*/bin

# install tensorflow
RUN apt-get update && apt-get install -y --no-install-recommends protobuf-compiler  \
  && apt-get clean \
  && rm -rf /var/lib/apt/lists/*
RUN pip3 install tensorflow-gpu==1.14.0 Cython contextlib2 matplotlib pillow \
                 lxml
RUN git clone https://github.com/tensorflow/models.git /root/tensorflow-models
RUN cd /root/tensorflow-models/research \
    && protoc object_detection/protos/*.proto --python_out=. \
    && cp -r /root/tensorflow-models /tensorflow-models
ENV PYTHONPATH=$PYTHONPATH:.:/tensorflow-models/research:/tensorflow-models/research/slim

# configure run user
RUN groupadd -r exopticon && useradd --no-log-init -m -g exopticon --uid 1000 exopticon
USER exopticon:exopticon

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y \
    && ~/.cargo/bin/rustup component add clippy \
    && ~/.cargo/bin/rustup update

# configure environment
ENV PYTHONPATH=$PYTHONPATH:/exopticon/exopticon/workers/yolov3
ENV PATH=/root/.cargo/bin:/exopticon/exopticon/workers:$PATH
ENV CUDA_HOME=/usr/local/cuda-10.0
ENV CUDA_PATH=/usr/local/cuda-10.0/bin

WORKDIR /exopticon

FROM gw000/debian-cuda:9.1_7.0

# install Exopticon dependencies

# install cuda
#RUN apt-get update && apt-get install -q -y nvidia-cuda-dev nvidia-cuda-toolkit

# install Python 3.7
RUN echo 'deb-src http://http.debian.net/debian stable main contrib non-free' > /etc/apt/sources.list.d/src.list \
&& apt-get update && apt-get install -y zlib1g-dev build-essential libsqlite3-dev sqlite3 bzip2 libbz2-dev zlib1g-dev \
                                        libssl-dev openssl libgdbm-dev liblzma-dev libreadline-dev libncursesw5-dev libffi-dev uuid-dev

#RUN apt-get build-dep -y python3 \
#&& wget -nv -P /root/src https://www.python.org/ftp/python/3.7.3/Python-3.7.3.tgz \
#&& cd /root/src/ && tar xf Python-3.7.3.tgz && cd Python-3.7.3 \
#&& ./configure --enable-optimizations --with-ensurepip=install \
#&& make -j4 \
#&& make altinstall \
#&& rm -r /root/src

# install dlib
## dlib dependencies
RUN apt-get update && apt-get install -y \
git \
cmake \
libsm6 \
libxext6 \
libxrender-dev \
libopenblas-dev \
liblapack-dev \
lsb-release

#RUN ln -sf /usr/local/bin/python3.7 /usr/bin/python3 \
#    && ln -sf /usr/local/bin/pip3.7 /usr/bin/pip3 \
#    && ln -s /usr/share/pyshared/lsb_release.py /usr/local/lib/python3.7/site-packages/
#RUN pip3 install scikit-build

RUN apt-get install -y python3 python3-pip

## fetch dlib sources
#RUN git clone -b 'v19.16' --single-branch https://github.com/davisking/dlib.git
#RUN mkdir -p /dlib/build

#RUN cmake -H/dlib -B/dlib/build -DDLIB_USE_CUDA=1 -DUSE_AVX_INSTRUCTIONS=1
#RUN cmake --build /dlib/build
#RUN cd /dlib; python3 /dlib/setup.py install

# install lightnet
## install lightnet dependencies
#RUN pip3 install torch==0.4.1.post2
#RUN pip3 install torchvision==0.2.2
#RUN git clone https://gitlab.com/EAVISE/lightnet.git /root/lightnet
#RUN cd /root/lightnet && pip3 install -r develop.txt

# install tensorflow
## install 1.12.2 b/c is uses cuda 9
RUN pip3 install tensorflow-gpu==1.12.2 Cython contextlib2 matplotlib pillow \
                 lxml
RUN git clone https://github.com/tensorflow/models.git /root/tensorflow-models
RUN apt-get install -y protobuf-compiler python-pil python-lxml python-tk python3-tk
RUN cd /root/tensorflow-models/research \
    && protoc object_detection/protos/*.proto --python_out=.
ENV PYTHONPATH=$PYTHONPATH:.:/root/tensorflow-models/research:/root/tensorflow-models/research/slim

# install opencv
RUN apt-get build-dep -y opencv
RUN git clone --branch 4.1.0 https://github.com/opencv/opencv.git /root/opencv \
    && cd /root/opencv \
    && mkdir build && cd build \
    && cmake .. \
    && make -j20 \
    && make install \
    && cd && rm -r /root/opencv

# install Exopticon dependencies
RUN apt-get install -yq libavcodec-dev libavformat-dev libswscale-dev libavfilter-dev \
           libavutil-dev libturbojpeg0-dev autoconf automake bzip2 \
           dpkg-dev file g++ gcc imagemagick libbz2-dev libc6-dev \
           libcurl4-openssl-dev libdb-dev libevent-dev libffi-dev\
           libgdbm-dev libgeoip-dev libglib2.0-dev libjpeg-dev \
           libkrb5-dev liblzma-dev libmagickcore-dev libmagickwand-dev\
           libncurses5-dev libncursesw5-dev libpng-dev libpq-dev \
           libreadline-dev libsqlite3-dev libssl-dev libtool libwebp-dev \
           libxml2-dev libxslt-dev libyaml-dev make patch xz-utils \
           zlib1g-dev default-libmysqlclient-dev libturbojpeg0-dev \
           curl

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y

RUN pip3 install msgpack

RUN apt-get install -y libcudnn7=7.1.4.18-1+cuda9.0 libcudnn7-dev=7.1.4.18-1+cuda9.0

# configure environment
ENV PATH=/root/.cargo/bin:/exopticon/workers:$PATH
ENV CUDA_HOME=/usr/local/cuda-9.0
ENV CUDA_PATH=/usr/local/cuda-9.0/bin

WORKDIR /exopticon



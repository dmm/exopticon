FROM gw000/debian-cuda:9.1_7.0

#RUN echo 'deb http://deb.debian.org/debian unstable contrib non-free' > /etc/apt/sources.list.d/nonfree.list

# install Exopticon dependencies

# install cuda
#RUN apt-get update && apt-get install -q -y nvidia-cuda-dev nvidia-cuda-toolkit

# install dlib
## dlib dependencies
RUN apt-get update && apt-get install -y \
git \
cmake \
libsm6 \
libxext6 \
libxrender-dev \
python3 \
python3-pip \
libopenblas-dev \
liblapack-dev

RUN pip3 install scikit-build

## fetch dlib sources
RUN git clone -b 'v19.16' --single-branch https://github.com/davisking/dlib.git
RUN mkdir -p /dlib/build

RUN cmake -H/dlib -B/dlib/build -DDLIB_USE_CUDA=1 -DUSE_AVX_INSTRUCTIONS=1
RUN cmake --build /dlib/build
RUN cd /dlib; python3 /dlib/setup.py install


# install lightnet
## install lightnet dependencies
#RUN pip3 install torch torchvision

# install opencv
#RUN apt-get install python3-opencv

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
ENV PATH=/root/.cargo/bin:$PATH

WORKDIR /exopticon



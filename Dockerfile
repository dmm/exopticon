FROM docker.io/almalinux:10 as base

# Install EPEL
RUN  dnf install -y https://dl.fedoraproject.org/pub/epel/epel-release-latest-10.noarch.rpm

ENV CUDA_VERSION 13.0.1
ENV CUDA_MAJOR 13
ENV CUDA_MINOR 0
ENV CUDA_REPO_URL https://developer.download.nvidia.com/compute/cuda/repos/rhel10
ENV NV_CUDA_CUDART_VERSION 13.0.88-1

RUN echo "[cuda]" > /etc/yum.repos.d/cuda.repo
RUN echo "name=cuda" >> /etc/yum.repos.d/cuda.repo
RUN echo "baseurl=https://developer.download.nvidia.com/compute/cuda/repos/rhel10/x86_64" >> /etc/yum.repos.d/cuda.repo
RUN echo "enabled=1" >> /etc/yum.repos.d/cuda.repo
RUN echo "gpgcheck=1" >> /etc/yum.repos.d/cuda.repo
RUN echo "gpgkey=file:///etc/pki/rpm-gpg/RPM-GPG-KEY-NVIDIA" >> /etc/yum.repos.d/cuda.repo

LABEL maintainer "David Matthew Mattli <dmm@mattli.us>"
RUN NVIDIA_GPGKEY_SUM=afbea87d3b979b3788ef34223aeeb323ade481128e2c133723ae99b8a51368bb && \
    curl -fsSL https://developer.download.nvidia.com/compute/cuda/repos/rhel10/x86_64/CDF6BA43.pub | sed '/^Version/d' > /etc/pki/rpm-gpg/RPM-GPG-KEY-NVIDIA && \
    echo "$NVIDIA_GPGKEY_SUM  /etc/pki/rpm-gpg/RPM-GPG-KEY-NVIDIA" | sha256sum -c --strict -

RUN dnf upgrade -y \
    && dnf install -y cuda-cudart-13-0 cuda-compat-13-0 \
    && dnf clean all \
    && rm -rf /var/cache/yum/*
# For libraries in the cuda-compat-* package: https://docs.nvidia.com/cuda/eula/index.html#attachment-a
# RUN yum upgrade -y && yum install -y \
#     cuda-cudart-${CUDA_MAJOR}-${CUDA_MINOR}-${NV_CUDA_CUDART_VERSION} \
#     cuda-compat-${CUDA_MAJOR}-${CUDA_MINOR \
# {% if ( cuda.version.major | int ) == 11 and ( cuda.version.minor | int ) <= 2 %}
#     && ln -s cuda-{{ cuda.version.major }}.{{ cuda.version.minor }} /usr/local/cuda \
# {% endif %}
#     && yum clean all \
#     && rm -rf /var/cache/yum/*

# nvidia-docker 1.0
RUN echo "/usr/local/nvidia/lib" >> /etc/ld.so.conf.d/nvidia.conf && \
    echo "/usr/local/nvidia/lib64" >> /etc/ld.so.conf.d/nvidia.conf

ENV PATH /usr/local/nvidia/bin:/usr/local/cuda/bin:${PATH}
ENV LD_LIBRARY_PATH /usr/local/nvidia/lib:/usr/local/nvidia/lib64

# nvidia-container-runtime
ENV NVIDIA_VISIBLE_DEVICES all
ENV NVIDIA_DRIVER_CAPABILITIES compute,utility

FROM base as cuda-runtime

RUN dnf install -y \
    cuda-libraries-13-0 \
    cuda-nvtx-13-0 \
    libnpp-13-0 \
    libcublas-13-0 \
    libnccl \
    cudnn \
    && dnf clean all \
    && rm -rf /var/cache/yum/*


FROM cuda-runtime as cuda-devel

RUN dnf install -y \
    cuda-command-line-tools-13-0 \
    cuda-libraries-devel-13-0 \
    cuda-minimal-build-13-0 \
    cuda-cudart-devel-13-0 \
    cuda-nvml-devel-13-0 \
    libcublas-devel-13-0 \
    libnpp-devel-13-0 \
    libnccl-devel \
    libcudnn9-devel-cuda-13 \
    libcudnn9-headers-cuda-13 \
    && dnf clean all \
    && rm -rf /var/cache/yum/*

FROM cuda-devel as exopticon-build

WORKDIR /exopticon

RUN dnf install -y \
    make \
    mold \
    clang \
    git \
    gstreamer1 \
    gstreamer1-devel \
    gstreamer1-plugins-bad-free-devel \
    gstreamer1-plugins-base-devel \
    gstreamer1-plugins-good \
    gstreamer1-plugins-ugly-free \
    libpq-devel \
    nodejs \
    nodejs-npm \
    python3-pip \
    # for Rust openssl crate:
    pkgconf \
    perl-FindBin \
    perl-IPC-Cmd \
    openssl-devel \
    perl-File-Compare \
    perl-File-Copy \
    perl-Time-Piece \
    && dnf clean all \
    && rm -rf /var/cache/yum/*


RUN mkdir /cargo && mkdir /rust
RUN chown 1000:1000 /cargo /rust

# configure run user
RUN groupadd -r -g 1000 exopticon && useradd --no-log-init -m -g exopticon --uid 1000 exopticon
RUN chown exopticon:exopticon /exopticon

USER exopticon:exopticon

ENV CARGO_HOME=/cargo
ENV RUST_HOME=/rust

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
  && /cargo/bin/rustup toolchain install 1.90.0 \
  && /cargo/bin/rustup default 1.90.0 \
  && /cargo/bin/rustup component add clippy

RUN pip3 install msgpack imutils numpy pathspec==0.9.0 dvc[s3]==1.11.16 importlib-metadata
RUN /home/exopticon/.local/bin/dvc config --global core.analytics false

ENV EXOPTICONWORKERS=/target/debug/
#ENV PYTHONPATH=$EXOPTICONWORKERS:/opt/opencv/lib/python3.7/dist-packages
ENV CUDA_HOME=/usr/local/cuda-13.0
ENV CUDA_PATH=/usr/local/cuda-13.0/bin
ENV CUDA_TOOLKIT_DIR=/usr/local/cuda-13.0
ENV CUDACXX=/usr/local/cuda-13.0/bin/nvcc
ENV PATH=$CUDA_PATH:/exopticon/target/debug:$CARGO_HOME/bin:/exopticon/exopticon/workers:/home/exopticon/.local/bin/:$PATH

FROM exopticon-build as exopticon-development

ENTRYPOINT ["sleep", "infinity"]

FROM exopticon-build as prod-build

USER exopticon:exopticon

COPY --chown=exopticon:exopticon . ./

RUN make ci-flow

FROM cuda-runtime as exopticon-prod

WORKDIR /exopticon

USER root

RUN dnf install -y \
    gstreamer1 \
    gstreamer1-plugins-bad-free \
    gstreamer1-plugins-base \
    gstreamer1-plugins-good \
    gstreamer1-plugins-ugly-free \
    libpq \
    # for Rust openssl crate:
    openssl \
    && dnf clean all \
    && rm -rf /var/cache/yum/*

FROM exopticon-prod AS exopticon-cuda

WORKDIR /exopticon

# configure run user
RUN groupadd -r -g 1000 exopticon && useradd --no-log-init -m -g exopticon --uid 1000 exopticon

COPY --chown=exopticon:exopticon --from=prod-build /exopticon/target/release/exopticon .

COPY --chown=exopticon:exopticon --from=prod-build /exopticon/target/release/capture_worker .

ENV EXOPTICONWORKERS=/exopticon/
ENV PATH=/exopticon:$PATH
ENV LD_LIBRARY_PATH=/usr/local/lib

USER exopticon:exopticon

ENTRYPOINT /exopticon/exopticon

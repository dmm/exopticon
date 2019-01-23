# EXOPTICON

Video Surveillance

## Installation

1. Install dependencies

Install Rust

    curl https://sh.rustup.rs -sSf | sh

Install system dependencies:

    sudo apt install -y libavcodec-dev libavformat-dev libswscale-dev libavfilter-dev \
               libavutil-dev libturbojpeg0-dev autoconf automake bzip2 \
               dpkg-dev file g++ gcc imagemagick libbz2-dev libc6-dev \
               libcurl4-openssl-dev libdb-dev libevent-dev libffi-dev\
               libgdbm-dev libgeoip-dev libglib2.0-dev libjpeg-dev \
               libkrb5-dev liblzma-dev libmagickcore-dev libmagickwand-dev\
               libncurses5-dev libncursesw5-dev libpng-dev libpq-dev \
               libreadline-dev libsqlite3-dev libssl-dev libtool libwebp-dev \
               libxml2-dev libxslt-dev libyaml-dev make patch xz-utils \
               zlib1g-dev libmysqlclient-dev

Install Node.js:

    install node.js here

Install diesel cli

    cargo install diesel_cli


2. Build EXOPTICON

    cargo build --release


#!/bin/bash
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
OLD_PWD=$PWD

cd $DIR
mkdir build
cd build
cmake ../darknet
make -j10
cd ..
cp build/libdark.so data
wget -c -P data https://github.com/AlexeyAB/darknet/releases/download/darknet_yolo_v3_optimal/yolov4.weights
cd $OLD_PWD

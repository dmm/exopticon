#!/bin/bash
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
OLD_PWD=$PWD
OUTDIR="$DIR/../dist/yolov4"



cd $DIR
mkdir -p build
cd build
cmake ../darknet
make -j10
cd ..
cp build/libdark.so data
wget -c -P data https://github.com/AlexeyAB/darknet/releases/download/darknet_yolo_v3_optimal/yolov4.weights

mkdir -p "$OUTDIR/data"
cp -r data "$OUTDIR"

cp darknet.py "$OUTDIR"

cd $OLD_PWD

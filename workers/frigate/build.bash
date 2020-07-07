#!/bin/bash
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
OLD_PWD=$PWD
OUTDIR="$DIR/../dist/frigate"

cd "$DIR"
mkdir -p "$OUTDIR"/data

cp frigate/frigate/motion.py "$OUTDIR"/data/

cp -r motion.py "$OUTDIR"

cd "$OLD_PWD"

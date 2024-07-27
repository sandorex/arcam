#!/usr/bin/env bash
#
# simple installation script for box

set -eu

PREFIX="$HOME/.local"

echo "Installing box to $PREFIX"

mkdir -p "$PREFIX/bin/"

# copy all files
cp ./box-* "$PREFIX/bin/"

# TODO move boxes to .local/share/ or .config/box/
cp -r ./boxes "$PREFIX/bin/"

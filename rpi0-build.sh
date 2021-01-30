#!/bin/bash
set -ex

export PKG_CONFIG_SYSROOT_DIR=/usr/arm-linux-gnueabihf
export PKG_CONFIG_PATH=/usr/arm-linux-gnueabihf/lib
export RUSTFLAGS='-C link-arg=-lopus -C link-arg=-lstdc++ -L.' #-rpath-link=/usr/arm-linux-gnueabihf/lib'

cargo build --release --target arm-unknown-linux-gnueabihf --features ledpanel  --no-default-features
#cargo build --release --example matrix-with-audio --features ledpanel --no-default-features
if [ $1 ]; then
	scp target/arm-unknown-linux-gnueabihf/release/visualizer $1
fi

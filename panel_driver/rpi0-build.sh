#!/bin/bash
set -ex

export PKG_CONFIG_SYSROOT_DIR=/usr/arm-linux-gnueabihf
export PKG_CONFIG_PATH=/usr/arm-linux-gnueabihf/lib
export RUSTFLAGS='-C link-arg=-lopus -C link-arg=-lstdc++ -L../' #-rpath-link=/usr/arm-linux-gnueabihf/lib'

cargo build --bin $1 --release --target arm-unknown-linux-gnueabihf 
#cargo build --release --example matrix-with-audio --features ledpanel --no-default-features
if [ $2 ]; then
	scp ../target/arm-unknown-linux-gnueabihf/release/$1 $2
fi

#!/bin/sh
# Run only inside the documented disposable Linux container.
set -eu
export DEBIAN_FRONTEND=noninteractive
apt-get update -qq
apt-get install -y -qq --no-install-recommends python3 > /qa/setup.log
mkdir -p /work/baseline
cp -a /baseline/. /work/baseline/
cp /audit/Cargo.lock.baseline /work/baseline/Cargo.lock
cd /work/baseline
rustc --version > /qa/tool-versions.txt
cargo --version >> /qa/tool-versions.txt
cc --version | head -n 1 >> /qa/tool-versions.txt
cargo build --release --locked > /qa/build.log 2>&1
cargo test --locked > /qa/cargo-test.log 2>&1
python3 /audit/audit.py /work/baseline/target/release/crexe /baseline/examples/calculator_native.crexe

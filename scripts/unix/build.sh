#!/usr/bin/env sh
set -eu
script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
exec cargo build --release --locked --manifest-path "$script_dir/../../Cargo.toml"

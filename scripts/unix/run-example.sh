#!/usr/bin/env sh
set -eu
script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
sh "$script_dir/build.sh"
exec "$script_dir/../../target/release/crexe" exec "$script_dir/../../examples/yaml/calculator_native.crexe" "$@"

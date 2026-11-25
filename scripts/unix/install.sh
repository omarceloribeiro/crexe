#!/usr/bin/env sh
set -eu
script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
sh "$script_dir/build.sh"
"$script_dir/../../target/release/crexe" install
exec sh "$script_dir/configure.sh"

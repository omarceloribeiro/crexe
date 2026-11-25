#!/usr/bin/env sh
set -eu
script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
if [ "$(uname -s)" = Darwin ]; then
    default_root="$HOME/Library/Application Support/CREXE"
else
    default_root="${XDG_DATA_HOME:-$HOME/.local/share}/crexe"
fi
engine="${CREXE_HOME:-$default_root}/releases/v1/crexe"
if [ ! -x "$engine" ]; then engine="$script_dir/../../target/release/crexe"; fi
exec "$engine" configure "$@"
